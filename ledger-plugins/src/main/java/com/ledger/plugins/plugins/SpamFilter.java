package com.ledger.plugins.plugins;

import com.ledger.plugins.Message;
import com.ledger.plugins.MessagePlugin;

import java.util.Arrays;
import java.util.List;

/**
 * Simple keyword-based spam filter plugin.
 */
public class SpamFilter implements MessagePlugin {
    private final List<String> spamKeywords = Arrays.asList(
            "viagra", "lottery", "winner", "prince", "inheritance",
            "click here", "free money", "congratulations", "urgent"
    );

    @Override
    public String getName() {
        return "SpamFilter";
    }

    @Override
    public boolean process(Message message) {
        String content = (message.subject + " " + message.body).toLowerCase();
        for (String keyword : spamKeywords) {
            if (content.contains(keyword)) {
                System.out.printf("[SpamFilter] Blocked message '%s' (matched: %s)%n",
                        message.subject, keyword);
                return false;
            }
        }
        return true;
    }
}
