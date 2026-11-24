param(
    [string]$Serial = 'emulator-5554',
    [string]$Binary = (Join-Path $PSScriptRoot '../target/x86_64-linux-android/release/agy-droidbridge')
)

$ErrorActionPreference = 'Stop'
$Binary = (Resolve-Path -LiteralPath $Binary).Path
$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { $env:ANDROID_SDK_ROOT }
$adb = Join-Path $sdk 'platform-tools/adb.exe'
$guest = '/data/local/tmp/droidbridge-idle-' + [guid]::NewGuid().ToString('N')
$port = 18776

function Invoke-Adb {
    & $adb -s $Serial @args
    if ($LASTEXITCODE -ne 0) { throw "adb failed with exit code $LASTEXITCODE" }
}

$android = Invoke-Adb shell getprop ro.build.version.release
$abi = Invoke-Adb shell getprop ro.product.cpu.abi
$start = 'TMPDIR={0} AGY_BIN=/system/bin/false AGY_API_KEY=idle-benchmark AGY_PORT={1} nohup {0}/bridge >{0}/bridge.log 2>&1 </dev/null & echo $!' -f $guest, $port
$hostPort = $null
try {
    Invoke-Adb shell "mkdir -m 700 '$guest'"
    Invoke-Adb push $Binary "$guest/bridge" | Out-Host
    Invoke-Adb shell "chmod 700 '$guest/bridge'"
    $hostPort = Invoke-Adb forward tcp:0 "tcp:$port"
    if ($hostPort -notmatch '^\d+$') { throw 'adb did not assign a forwarding port' }

    $samples = foreach ($run in 1..3) {
        $bridgePid = Invoke-Adb shell $start
        if ($bridgePid -notmatch '^\d+$') { throw 'Android did not return a bridge PID' }
        try {
            $ready = $false
            foreach ($attempt in 1..20) {
                Invoke-Adb shell "kill -0 $bridgePid"
                try {
                    $health = Invoke-RestMethod "http://127.0.0.1:$hostPort/healthz" -TimeoutSec 1
                    $ready = $health.ok -eq $true -and $health.service -eq 'agy-droidbridge'
                } catch {
                    Start-Sleep -Milliseconds 100
                }
                if ($ready) { break }
            }
            if (!$ready) { throw 'Bridge did not become healthy' }

            Start-Sleep -Seconds 1
            foreach ($sample in 1..5) {
                $rss = Invoke-Adb shell "grep '^VmRSS:' /proc/$bridgePid/status"
                if ($rss -notmatch '^VmRSS:\s+(\d+)\s+kB\s*$') { throw "Unexpected RSS reading: $rss" }
                [pscustomobject]@{ Run = $run; Sample = $sample; RSSKiB = [int]$Matches[1] }
                if ($sample -lt 5) { Start-Sleep -Milliseconds 250 }
            }
        } finally {
            & $adb -s $Serial shell "kill -TERM $bridgePid" 2>$null
            Start-Sleep -Seconds 1
        }
    }

    $sortedRss = @($samples.RSSKiB | Sort-Object)
    $middle = [int][math]::Floor($sortedRss.Count / 2)
    [pscustomobject]@{
        Android = $android
        ABI = $abi
        BinaryBytes = (Get-Item -LiteralPath $Binary).Length
        MedianRSSKiB = $sortedRss[$middle]
        MinRSSKiB = $sortedRss[0]
        MaxRSSKiB = $sortedRss[-1]
        Samples = $samples
    } | ConvertTo-Json -Depth 3
} finally {
    if ($hostPort -match '^\d+$') { & $adb -s $Serial forward --remove "tcp:$hostPort" }
    & $adb -s $Serial shell "rm -f '$guest/bridge' '$guest/bridge.log' && rmdir '$guest'"
}
