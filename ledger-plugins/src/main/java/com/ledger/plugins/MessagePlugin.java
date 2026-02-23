package com.ledger.plugins;

/**
 * Plugin interface for Ledger message processors.
 * Implement this interface to create custom filters, taggers, or transformers.
 */
public interface MessagePlugin {

    /**
     * @return Plugin name for display / logging
     */
    String getName();

    /**
     * Process a message. Return true to keep the message, false to discard/filter it.
     *
     * @param message The message to process (can be mutated in-place for tagging)
     * @return true if message should be kept, false to filter out
     */
    boolean process(Message message);
}
