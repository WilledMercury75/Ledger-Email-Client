package com.ledger.plugins.plugins;

import com.ledger.plugins.Message;
import com.ledger.plugins.MessagePlugin;

/**
 * Auto-tagging plugin: prepends tags to message subjects based on delivery method.
 */
public class AutoTagger implements MessagePlugin {

    @Override
    public String getName() {
        return "AutoTagger";
    }

    @Override
    public boolean process(Message message) {
        if (message.deliveryMethod == null) return true;

        switch (message.deliveryMethod) {
            case "p2p":
                if (!message.subject.startsWith("[P2P]")) {
                    message.subject = "[P2P] " + message.subject;
                }
                break;
            case "gmail":
                if (!message.subject.startsWith("[Gmail]")) {
                    message.subject = "[Gmail] " + message.subject;
                }
                break;
            case "fallback":
                if (!message.subject.startsWith("[Fallback]")) {
                    message.subject = "[Fallback] " + message.subject;
                }
                break;
        }

        if (message.encrypted) {
            if (!message.subject.startsWith("[🔒]")) {
                message.subject = "[🔒] " + message.subject;
            }
        }

        System.out.printf("[AutoTagger] Tagged: %s%n", message.subject);
        return true;
    }
}
