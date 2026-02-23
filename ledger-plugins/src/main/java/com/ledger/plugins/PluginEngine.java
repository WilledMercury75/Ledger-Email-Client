package com.ledger.plugins;

import com.ledger.plugins.plugins.AutoTagger;
import com.ledger.plugins.plugins.SpamFilter;

import java.util.ArrayList;
import java.util.List;

/**
 * Plugin engine that connects to the Ledger Core API,
 * fetches messages, and runs them through the plugin pipeline.
 */
public class PluginEngine {
    private final LedgerApiClient api;
    private final List<MessagePlugin> plugins;

    public PluginEngine(String apiUrl) {
        this.api = new LedgerApiClient(apiUrl);
        this.plugins = new ArrayList<>();
    }

    public PluginEngine() {
        this("http://127.0.0.1:8420");
    }

    /**
     * Register a plugin.
     */
    public void registerPlugin(MessagePlugin plugin) {
        plugins.add(plugin);
        System.out.printf("Registered plugin: %s%n", plugin.getName());
    }

    /**
     * Process all inbox messages through the plugin pipeline.
     * Returns messages that passed all filters.
     */
    public List<Message> processInbox() throws Exception {
        List<Message> inbox = api.getInbox();
        System.out.printf("Fetched %d inbox messages%n", inbox.size());

        List<Message> processed = new ArrayList<>();
        for (Message msg : inbox) {
            boolean keep = true;
            for (MessagePlugin plugin : plugins) {
                if (!plugin.process(msg)) {
                    keep = false;
                    break;
                }
            }
            if (keep) {
                processed.add(msg);
            }
        }

        System.out.printf("After processing: %d messages kept, %d filtered%n",
                processed.size(), inbox.size() - processed.size());
        return processed;
    }

    /**
     * Entry point: demonstrates plugin engine with built-in plugins.
     */
    public static void main(String[] args) {
        String apiUrl = args.length > 0 ? args[0] : "http://127.0.0.1:8420";

        System.out.println("╔══════════════════════════════════════╗");
        System.out.println("║    LEDGER PLUGIN ENGINE v0.1.0       ║");
        System.out.println("╠══════════════════════════════════════╣");
        System.out.println("║  Connecting to: " + apiUrl);
        System.out.println("╚══════════════════════════════════════╝");

        PluginEngine engine = new PluginEngine(apiUrl);

        // Register built-in plugins
        engine.registerPlugin(new SpamFilter());
        engine.registerPlugin(new AutoTagger());

        try {
            // Check API connectivity
            var identity = engine.api.getIdentity();
            if (identity != null) {
                System.out.println("Connected to Ledger Core: " + identity.get("ledger_id"));
            } else {
                System.err.println("Failed to connect to Ledger Core at " + apiUrl);
                System.err.println("Make sure ledger-core is running.");
                return;
            }

            // Process inbox
            List<Message> messages = engine.processInbox();
            for (Message msg : messages) {
                System.out.printf("  → %s | %s | %s%n", msg.deliveryMethod, msg.subject, msg.fromId);
            }
        } catch (Exception e) {
            System.err.println("Error: " + e.getMessage());
            System.err.println("Make sure ledger-core is running on " + apiUrl);
        }
    }
}
