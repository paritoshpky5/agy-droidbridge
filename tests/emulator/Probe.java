package dev.mobilelab.agyprobe;

import android.app.Activity;
import android.app.Instrumentation;
import android.os.Bundle;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;

public final class Probe extends Instrumentation {
    @Override
    public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        start();
    }

    @Override
    public void onStart() {
        Bundle results = new Bundle();
        try {
            JSONObject health = new JSONObject(request("/healthz", null, false, 200));
            if (!health.getBoolean("ok")) throw new AssertionError("health failed");
            request("/v1/models", null, false, 401);
            JSONObject models = new JSONObject(request("/v1/models", null, true, 200));
            String model = models.getJSONArray("data").getJSONObject(0).getString("id");
            String body = "{\"model\":" + JSONObject.quote(model)
                + ",\"messages\":[{\"role\":\"user\",\"content\":\"Android localhost\"}]}";
            JSONObject completion = new JSONObject(request("/v1/chat/completions", body, true, 200));
            String text = completion.getJSONArray("choices").getJSONObject(0)
                .getJSONObject("message").getString("content");
            if (!text.contains("Android localhost")) throw new AssertionError("unexpected completion");
            results.putString("stream", "Android app passed localhost health, unauthorized, models, and chat completion");
            finish(Activity.RESULT_OK, results);
        } catch (Exception | AssertionError failure) {
            results.putString("stream", failure.toString());
            finish(Activity.RESULT_CANCELED, results);
        }
    }

    private String request(String path, String body, boolean authenticated, int expected) throws Exception {
        HttpURLConnection connection = (HttpURLConnection)
            new URL("http://127.0.0.1:8766" + path).openConnection();
        try {
            connection.setConnectTimeout(3000);
            connection.setReadTimeout(8000);
            if (authenticated) connection.setRequestProperty("Authorization", "Bearer emulator-test-key");
            if (body != null) {
                connection.setRequestMethod("POST");
                connection.setDoOutput(true);
                connection.setRequestProperty("Content-Type", "application/json");
                try (OutputStream output = connection.getOutputStream()) {
                    output.write(body.getBytes(StandardCharsets.UTF_8));
                }
            }
            int status = connection.getResponseCode();
            if (status != expected) throw new AssertionError("HTTP " + status + " for " + path);
            try (InputStream input = status < 400 ? connection.getInputStream() : connection.getErrorStream()) {
                ByteArrayOutputStream output = new ByteArrayOutputStream();
                byte[] buffer = new byte[4096];
                int read;
                while ((read = input.read(buffer)) != -1) output.write(buffer, 0, read);
                return new String(output.toByteArray(), StandardCharsets.UTF_8);
            }
        } finally {
            connection.disconnect();
        }
    }
}
