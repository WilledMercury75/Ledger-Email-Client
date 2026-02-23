using System;
using System.Collections.ObjectModel;
using System.Threading.Tasks;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LedgerUI.Services;

namespace LedgerUI.ViewModels;

public partial class MainWindowViewModel : ObservableObject
{
    private readonly LedgerApiClient _api = new();

    [ObservableProperty] private string _currentView = "Inbox";
    [ObservableProperty] private string _ledgerId = "Loading...";
    [ObservableProperty] private string _peerId = "";
    [ObservableProperty] private string _statusMessage = "Connecting to Ledger Core...";
    [ObservableProperty] private bool _isConnected;

    // Messages
    [ObservableProperty] private ObservableCollection<MessageDto> _messages = new();
    [ObservableProperty] private MessageDto? _selectedMessage;

    // Compose
    [ObservableProperty] private string _composeTo = "";
    [ObservableProperty] private string _composeSubject = "";
    [ObservableProperty] private string _composeBody = "";
    [ObservableProperty] private string _composeMode = "auto";

    // Settings
    [ObservableProperty] private string _gmailEmail = "";
    [ObservableProperty] private string _gmailAppPassword = "";
    [ObservableProperty] private string _deliveryMode = "auto";
    [ObservableProperty] private bool _torEnabled;
    [ObservableProperty] private bool _gmailConfigured;

    // Peers
    [ObservableProperty] private string _connectAddress = "";
    [ObservableProperty] private ObservableCollection<PeerDto> _peers = new();

    public MainWindowViewModel()
    {
        _ = InitializeAsync();
    }

    private async Task InitializeAsync()
    {
        try
        {
            var identity = await _api.GetIdentityAsync();
            if (identity != null)
            {
                LedgerId = identity.LedgerId;
                PeerId = identity.PeerId;
                IsConnected = true;
                StatusMessage = "Connected to Ledger Core";
            }
            await LoadInboxAsync();
            await LoadSettingsAsync();
        }
        catch (Exception ex)
        {
            StatusMessage = $"Connection failed: {ex.Message}";
            IsConnected = false;
        }
    }

    // ── Navigation ──

    [RelayCommand]
    private async Task NavigateTo(string view)
    {
        CurrentView = view;
        switch (view)
        {
            case "Inbox": await LoadMessagesAsync("inbox"); break;
            case "Sent": await LoadMessagesAsync("sent"); break;
            case "Drafts": await LoadMessagesAsync("drafts"); break;
            case "Settings": await LoadSettingsAsync(); break;
            case "Peers": await LoadPeersAsync(); break;
        }
    }

    // ── Messages ──

    private async Task LoadInboxAsync() => await LoadMessagesAsync("inbox");

    private async Task LoadMessagesAsync(string folder)
    {
        try
        {
            var msgs = await _api.GetMessagesAsync(folder);
            Messages.Clear();
            if (msgs != null)
                foreach (var m in msgs) Messages.Add(m);
        }
        catch (Exception ex)
        {
            StatusMessage = $"Failed to load messages: {ex.Message}";
        }
    }

    [RelayCommand]
    private async Task RefreshMessages()
    {
        var folder = CurrentView.ToLowerInvariant();
        if (folder == "compose" || folder == "settings" || folder == "peers") folder = "inbox";
        await LoadMessagesAsync(folder);
        StatusMessage = "Messages refreshed";
    }

    [RelayCommand]
    private async Task DeleteMessage(string id)
    {
        await _api.DeleteMessageAsync(id);
        await RefreshMessages();
    }

    // ── Compose ──

    [RelayCommand]
    private async Task SendMessage()
    {
        if (string.IsNullOrWhiteSpace(ComposeTo)) { StatusMessage = "Recipient required"; return; }

        try
        {
            var result = await _api.SendMessageAsync(ComposeTo, ComposeSubject, ComposeBody, ComposeMode);
            if (result != null)
            {
                StatusMessage = $"Message sent via {result.DeliveryMethod}";
                ComposeTo = ""; ComposeSubject = ""; ComposeBody = "";
                CurrentView = "Sent";
                await LoadMessagesAsync("sent");
            }
        }
        catch (Exception ex)
        {
            StatusMessage = $"Send failed: {ex.Message}";
        }
    }

    // ── Settings ──

    private async Task LoadSettingsAsync()
    {
        try
        {
            var settings = await _api.GetSettingsAsync();
            if (settings != null)
            {
                DeliveryMode = settings.TryGetValue("delivery_mode", out var dm) ? dm : "auto";
                TorEnabled = settings.TryGetValue("tor_enabled", out var te) && te == "true";
            }

            var gmail = await _api.GetGmailConfigAsync();
            if (gmail != null)
            {
                GmailConfigured = gmail.Configured;
                GmailEmail = gmail.Email ?? "";
            }
        }
        catch { /* UI continues with defaults */ }
    }

    [RelayCommand]
    private async Task SaveGmailConfig()
    {
        try
        {
            await _api.SetGmailConfigAsync(GmailEmail, GmailAppPassword);
            GmailConfigured = true;
            GmailAppPassword = "";
            StatusMessage = "Gmail configured successfully";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Gmail config failed: {ex.Message}";
        }
    }

    [RelayCommand]
    private async Task SaveSettings()
    {
        try
        {
            await _api.UpdateSettingsAsync(DeliveryMode, TorEnabled);
            StatusMessage = "Settings saved";
        }
        catch (Exception ex)
        {
            StatusMessage = $"Settings save failed: {ex.Message}";
        }
    }

    [RelayCommand]
    private async Task FetchGmail()
    {
        try
        {
            await _api.FetchGmailAsync();
            StatusMessage = "Gmail messages fetched";
            await LoadMessagesAsync("inbox");
        }
        catch (Exception ex)
        {
            StatusMessage = $"Gmail fetch failed: {ex.Message}";
        }
    }

    // ── Peers ──

    private async Task LoadPeersAsync()
    {
        try
        {
            var p = await _api.GetPeersAsync();
            Peers.Clear();
            if (p != null)
                foreach (var peer in p) Peers.Add(peer);
        }
        catch { }
    }

    [RelayCommand]
    private async Task ConnectToPeer()
    {
        if (string.IsNullOrWhiteSpace(ConnectAddress)) return;
        try
        {
            await _api.ConnectPeerAsync(ConnectAddress);
            StatusMessage = $"Connecting to {ConnectAddress}";
            ConnectAddress = "";
            await LoadPeersAsync();
        }
        catch (Exception ex)
        {
            StatusMessage = $"Connect failed: {ex.Message}";
        }
    }
}
