package com.ledger.plugins;

/**
 * Message data model matching Rust API response
 */
public class Message {
    public String id;
    public String fromId;
    public String toId;
    public String subject;
    public String body;
    public long timestamp;
    public String deliveryMethod;
    public boolean isRead;
    public String folder;
    public boolean encrypted;

    @Override
    public String toString() {
        return String.format("Message{id=%s, from=%s, subject=%s, method=%s}",
                id, fromId, subject, deliveryMethod);
    }
}
