using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Net.Http.Json;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading.Tasks;

namespace LedgerUI.Services;

/// <summary>
/// HTTP client for the Ledger Core REST API
/// </summary>
public class LedgerApiClient : IDisposable
{
    private readonly HttpClient _http;
    private static readonly JsonSerializerOptions JsonOpts = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        PropertyNameCaseInsensitive = true,
    };

    public LedgerApiClient(string baseUrl = "http://127.0.0.1:8420")
    {
        _http = new HttpClient { BaseAddress = new Uri(baseUrl) };
        _http.Timeout = TimeSpan.FromSeconds(10);
    }

    // ── Identity ──

    public async Task<IdentityInfo?> GetIdentityAsync()
    {
        var resp = await _http.GetFromJsonAsync<ApiResponse<IdentityInfo>>("/api/identity", JsonOpts);
        return resp?.Data;
    }

    // ── Messages ──

    public async Task<List<MessageDto>?> GetMessagesAsync(string? folder = null)
    {
        var url = folder != null ? $"/api/messages?folder={folder}" : "/api/messages";
        var resp = await _http.GetFromJsonAsync<ApiResponse<List<MessageDto>>>(url, JsonOpts);
        return resp?.Data;
    }

    public async Task<MessageDto?> GetMessageAsync(string id)
    {
        var resp = await _http.GetFromJsonAsync<ApiResponse<MessageDto>>($"/api/messages/{id}", JsonOpts);
        return resp?.Data;
    }

    public async Task<MessageDto?> SendMessageAsync(string to, string subject, string body, string? mode = null)
    {
        var payload = new { to, subject, body, mode = mode ?? "auto" };
        var response = await _http.PostAsJsonAsync("/api/messages", payload, JsonOpts);
        var resp = await response.Content.ReadFromJsonAsync<ApiResponse<MessageDto>>(JsonOpts);
        return resp?.Data;
    }

    public async Task<bool> DeleteMessageAsync(string id)
    {
        var response = await _http.DeleteAsync($"/api/messages/{id}");
        return response.IsSuccessStatusCode;
    }

    // ── Peers ──

    public async Task<List<PeerDto>?> GetPeersAsync()
    {
        var resp = await _http.GetFromJsonAsync<ApiResponse<List<PeerDto>>>("/api/peers", JsonOpts);
        return resp?.Data;
    }

    public async Task<bool> ConnectPeerAsync(string multiaddr)
    {
        var response = await _http.PostAsJsonAsync("/api/peers", new { multiaddr }, JsonOpts);
        return response.IsSuccessStatusCode;
    }

    // ── Gmail ──

    public async Task<GmailConfigStatus?> GetGmailConfigAsync()
    {
        var resp = await _http.GetFromJsonAsync<ApiResponse<GmailConfigStatus>>("/api/gmail/config", JsonOpts);
        return resp?.Data;
    }

    public async Task<bool> SetGmailConfigAsync(string email, string appPassword)
    {
        var response = await _http.PostAsJsonAsync("/api/gmail/config",
            new { email, app_password = appPassword }, JsonOpts);
        return response.IsSuccessStatusCode;
    }

    public async Task<bool> FetchGmailAsync()
    {
        var response = await _http.PostAsync("/api/gmail/fetch", null);
        return response.IsSuccessStatusCode;
    }

    // ── Settings ──

    public async Task<Dictionary<string, string>?> GetSettingsAsync()
    {
        var resp = await _http.GetFromJsonAsync<ApiResponse<Dictionary<string, string>>>("/api/settings", JsonOpts);
        return resp?.Data;
    }

    public async Task<bool> UpdateSettingsAsync(string? deliveryMode = null, bool? torEnabled = null)
    {
        var response = await _http.PutAsJsonAsync("/api/settings",
            new { delivery_mode = deliveryMode, tor_enabled = torEnabled }, JsonOpts);
        return response.IsSuccessStatusCode;
    }

    // ── Contacts ──

    public async Task<bool> AddContactAsync(string ledgerId, string publicKey, string? displayName = null, string? gmailAddress = null)
    {
        var response = await _http.PostAsJsonAsync("/api/contacts",
            new { ledger_id = ledgerId, public_key = publicKey, display_name = displayName, gmail_address = gmailAddress },
            JsonOpts);
        return response.IsSuccessStatusCode;
    }

    public void Dispose() => _http.Dispose();
}

// ── DTOs ──

public class ApiResponse<T>
{
    public bool Success { get; set; }
    public T? Data { get; set; }
    public string? Error { get; set; }
}

public class IdentityInfo
{
    public string LedgerId { get; set; } = "";
    public string PublicKey { get; set; } = "";
    public string PeerId { get; set; } = "";
}

public class MessageDto
{
    public string Id { get; set; } = "";
    public string FromId { get; set; } = "";
    public string ToId { get; set; } = "";
    public string Subject { get; set; } = "";
    public string Body { get; set; } = "";
    public long Timestamp { get; set; }
    public string DeliveryMethod { get; set; } = "";
    public bool IsRead { get; set; }
    public string Folder { get; set; } = "";
    public bool Encrypted { get; set; }

    public string DeliveryIcon => DeliveryMethod switch
    {
        "p2p" => "🔒",
        "gmail" => "📧",
        "fallback" => "⚠️",
        _ => "❓"
    };

    public string FormattedDate => DateTimeOffset.FromUnixTimeSeconds(Timestamp).LocalDateTime.ToString("MMM dd, HH:mm");

    public string ShortFrom => FromId.Length > 20 ? FromId[..20] + "..." : FromId;
}

public class PeerDto
{
    public string PeerId { get; set; } = "";
    public string Address { get; set; } = "";
    public string? LedgerId { get; set; }
}

public class GmailConfigStatus
{
    public bool Configured { get; set; }
    public string? Email { get; set; }
}
