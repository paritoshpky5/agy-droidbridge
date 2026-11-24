param(
  [string]$Project = (Join-Path $PSScriptRoot '../..'),
  [string]$Serial = 'emulator-5554'
)
$ErrorActionPreference='Stop'
function Assert-LastAgyCommand {
  if ($LASTEXITCODE) { throw ('Verification command failed with code ' + $LASTEXITCODE) }
}
$Project=(Resolve-Path -LiteralPath $Project).Path
$probeRoot=Join-Path ([System.IO.Path]::GetTempPath()) ('AgyAndroidProbe-' + [guid]::NewGuid().ToString('N').Substring(0,8))
$classes=Join-Path $probeRoot 'classes'
New-Item -ItemType Directory -Force -Path $classes | Out-Null
$sdk=if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { $env:ANDROID_SDK_ROOT }
$bt=Join-Path $sdk 'build-tools/36.0.0'
$adb=Join-Path $sdk 'platform-tools/adb.exe'
$androidJar=Join-Path $sdk 'platforms/android-36/android.jar'
$ndkBin=Join-Path $sdk 'ndk/28.2.13676358/toolchains/llvm/prebuilt/windows-x86_64/bin'
$fixture=Join-Path $probeRoot 'mock-agy-android'
$java=Join-Path $Project 'tests/emulator/Probe.java'
$manifest=Join-Path $Project 'tests/emulator/AndroidManifest.xml'
$apk=Join-Path $probeRoot 'probe-unsigned.apk'
$aligned=Join-Path $probeRoot 'probe-aligned.apk'
$signed=Join-Path $probeRoot 'probe.apk'
$keyStoreDirectory=Join-Path $env:LOCALAPPDATA 'MobileLabTools/AgyAndroidProbe'
New-Item -ItemType Directory -Force -Path $keyStoreDirectory | Out-Null
$keyStore=Join-Path $keyStoreDirectory 'ephemeral-test-signing.jks'
$guest='/data/local/tmp/agy-droidbridge-' + [guid]::NewGuid().ToString('N').Substring(0,8)
$arch=(& $adb -s $serial shell getprop ro.product.cpu.abi).Trim()
if ($arch -ne 'x86_64') { throw 'Expected the existing x86_64 emulator' }
rustc --edition 2024 --target x86_64-linux-android -C "linker=$ndkBin/x86_64-linux-android24-clang.cmd" -O (Join-Path $Project 'tests/fixtures/mock_agy.rs') -o $fixture
Assert-LastAgyCommand
javac --release 8 -cp $androidJar -d $classes $java
Assert-LastAgyCommand
$bytecode=@(Get-ChildItem -Path $classes -File -Recurse -Filter '*.class' | ForEach-Object FullName)
& (Join-Path $bt 'd8.bat') --min-api 24 --lib $androidJar --output $probeRoot @bytecode
Assert-LastAgyCommand
& (Join-Path $bt 'aapt.exe') package -f -M $manifest -I $androidJar -F $apk
Assert-LastAgyCommand
jar uf $apk -C $probeRoot classes.dex
Assert-LastAgyCommand
& (Join-Path $bt 'zipalign.exe') -f 4 $apk $aligned
Assert-LastAgyCommand
if (!(Test-Path -LiteralPath $keyStore)) {
  keytool -genkeypair -keystore $keyStore -storepass lab-emulator-only -keypass lab-emulator-only -alias agy-probe -keyalg RSA -keysize 2048 -validity 30 -dname 'CN=MobileLab local emulator probe'
  Assert-LastAgyCommand
}
& (Join-Path $bt 'apksigner.bat') sign --ks $keyStore --ks-pass pass:lab-emulator-only --out $signed $aligned
Assert-LastAgyCommand
& $adb -s $serial install -r $signed
Assert-LastAgyCommand
& $adb -s $serial shell "mkdir -m 700 '$guest' && mkdir -m 700 '$guest/tmp'"
Assert-LastAgyCommand
& $adb -s $serial push $fixture "$guest/mock-agy"
Assert-LastAgyCommand
& $adb -s $serial push (Join-Path $Project 'target/x86_64-linux-android/release/agy-droidbridge') "$guest/agy-droidbridge"
Assert-LastAgyCommand
& $adb -s $serial shell "chmod 700 '$guest/mock-agy' '$guest/agy-droidbridge'"
Assert-LastAgyCommand
$startProxy = 'cd {0}; (TMPDIR={0}/tmp AGY_BIN={0}/mock-agy AGY_API_KEY=emulator-test-key AGY_WORKSPACE={0}/workspace nohup {0}/agy-droidbridge >{0}/proxy.log 2>&1 </dev/null & echo $! >{0}/proxy.pid)' -f $guest
$stopProxy = 'kill -TERM $(cat {0}/proxy.pid)' -f $guest
try {
  foreach ($attempt in 1..2) {
    & $adb -s $serial shell $startProxy
    Assert-LastAgyCommand
    Start-Sleep -Milliseconds 700
    & $adb -s $serial shell "kill -0 `$(cat $guest/proxy.pid)"
    Assert-LastAgyCommand
    $instrumentation=@(& $adb -s $serial shell am instrument -w -r dev.mobilelab.agyprobe/.Probe)
    Assert-LastAgyCommand
    $instrumentation
    if (($instrumentation -join "`n") -notmatch 'INSTRUMENTATION_CODE: -1' -or ($instrumentation -join "`n") -match 'INSTRUMENTATION_FAILED|INSTRUMENTATION_ABORTED') { throw 'Android instrumentation did not pass' }
    & $adb -s $serial shell $stopProxy
    Assert-LastAgyCommand
    Start-Sleep -Milliseconds 800
  }
} finally {
  & $adb -s $serial shell $stopProxy 2>$null | Out-Null
  & $adb -s $serial shell "cat '$guest/proxy.log'"
}
