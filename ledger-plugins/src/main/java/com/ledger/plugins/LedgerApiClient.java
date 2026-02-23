package com.ledger.plugins;

import com.google.gson.Gson;
import com.google.gson.JsonObject;
import com.google.gson.reflect.TypeToken;
import okhttp3.*;

import java.io.IOException;
import java.lang.reflect.Type;
import java.util.ArrayList;
import java.util.List;

/**
 * HTTP client for the Ledger Core REST API.
 */
public class LedgerApiClient {
    private final OkHttpClient http;
    private final String baseUrl;
    private final Gson gson;

    public LedgerApiClient(String baseUrl) {
        this.http = new OkHttpClient();
        this.baseUrl = baseUrl;
        this.gson = new Gson();
    }

    public LedgerApiClient() {
        this("http://127.0.0.1:8420");
    }

    /**
     * Fetch messages, optionally filtered by folder.
     */
    public List<Message> getMessages(String folder) throws IOException {
        String url = baseUrl + "/api/messages";
        if (folder != null) url += "?folder=" + folder;

        Request request = new Request.Builder().url(url).get().build();
        try (Response response = http.newCall(request).execute()) {
            if (!response.isSuccessful() || response.body() == null) {
                return new ArrayList<>();
            }
            String json = response.body().string();
            JsonObject wrapper = gson.fromJson(json, JsonObject.class);
            if (wrapper.has("data") && !wrapper.get("data").isJsonNull()) {
                Type listType = new TypeToken<List<Message>>() {}.getType();
                return gson.fromJson(wrapper.get("data"), listType);
            }
            return new ArrayList<>();
        }
    }

    /**
     * Fetch all inbox messages.
     */
    public List<Message> getInbox() throws IOException {
        return getMessages("inbox");
    }

    /**
     * Get identity info.
     */
    public JsonObject getIdentity() throws IOException {
        Request request = new Request.Builder()
                .url(baseUrl + "/api/identity")
                .get().build();
        try (Response response = http.newCall(request).execute()) {
            if (response.body() == null) return null;
            String json = response.body().string();
            JsonObject wrapper = gson.fromJson(json, JsonObject.class);
            return wrapper.has("data") ? wrapper.getAsJsonObject("data") : null;
        }
    }
}
