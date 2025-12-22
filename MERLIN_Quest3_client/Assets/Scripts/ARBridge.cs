using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;
using NativeWebSocket;
using Newtonsoft.Json;

/// Websocket client to AR Bridge...connecting to Jetson Backend

///Connection settings: Jetson IP & Port
/// Status: connection status, frames received, current FPS
public class ARBridge : MonoBehaviour
{
    [Header("Connection Settings")]
    [SerializeField] private string jetsonIp = "192.168.1.100"; //Jetson IP 
    [SerializeField] private int jetsonPort = 8765; //Jetson port
    [SerializeField] private float reconnectDelay = 2f; // Add delay between reconnections
    [SerializeField] private bool autoReconnect = true;

    [Header("Status")]
    public bool IsConnected { get; private set; } = false;
    [SerializeField] private int framesReceived = 0;
    public float CurrentFPS { get; private set; } = 0f;

    // Websocket connection
    private WebSocket websocket;
    private bool isQuitting = false; // Add quitting state for reconnections

    //Frame tracking
    private Queue<float> fpsHistory = new Queue<float>();
    private float lastFrameTime;

    //Events, notifications to listeners
    public event Action<ARFrame> OnFrameReceived;
    public event Action OnConnected;
    public event Action OnDisconnected;

    void Start()
    {
        Debug.Log($"Connecting to AR Bridge: ws://{jetsonIp}:{jetsonPort}");
        StartCoroutine(ConnectWithRetry());
    }

    void Update()
    {
        #if !UNITY_WEBGL || UNITY_EDITOR
        websocket?.DispatchMessageQueue();
        #endif
    }

    /// <summary>
    /// Need IEnumerator for coroutine to handle reconnections
    /// </summary>
    private IEnumerator ConnectWithRetry()
    {
        while (!isQuitting)
        {
            if (!IsConnected)
            {
                Debug.Log($"Attempting connection to ws://{jetsonIp}:{jetsonPort}");
                yield return StartCoroutine(AttemptConnection());
            }

            if (!IsConnected && autoReconnect)
            {
                Debug.Log($"Retrying in {reconnectDelay} seconds...");
                yield return new WaitForSeconds(reconnectDelay);
            }
            else if (IsConnected)
            {
                yield return new WaitForSeconds(1f);
            }
            else
            {
                break;
            }
        }
    }

    private IEnumerator AttemptConnection()
    {
        if (websocket != null)
        {
            websocket.OnOpen -= OnWebSocketOpen;
            websocket.OnMessage -= OnWebSocketMessage;
            websocket.OnError -= OnWebSocketError;
            websocket.OnClose -= OnWebSocketClose;

            if (websocket.State == WebSocketState.Open || websocket.State == WebSocketState.Connecting)
            {
                yield return websocket.Close();
            }
        }
        websocket = new WebSocket($"ws://{jetsonIp}:{jetsonPort}");
         
         //register callbacks
         websocket.OnOpen += OnWebSocketOpen;
         websocket.OnMessage += OnWebSocketMessage;
         websocket.OnError += OnWebSocketError;
         websocket.OnClose += OnWebSocketClose;

         var connectTask = websocket.Connect();
         float timeout = 5f;
         float elapsed = 0f;

         while (websocket.State == WebSocketState.Connecting && elapsed < timeout)
        {
            elapsed += Time.deltaTime;
            yield return null;
        }

        if (websocket.State != WebSocketState.Open)
        {
            Debug.LogWarning("Failed to connect to AR Bridge: Timeout");
        }
    }

    private void OnWebSocketOpen() //Connection Opened
    {
        IsConnected = true;
        Debug.Log("Connected to AR Bridge");
        OnConnected?.Invoke();
        SendClientConnect();
    }

    private void OnWebSocketMessage(byte[] data)
    {
        string json = System.Text.Encoding.UTF8.GetString(data);
        try
        {
            var message = JsonConvert.DeserializeObject<ServerMessage>(json);
            if (message?.Frame != null)
            {
                framesReceived++; //sum received frames

                //Calc FPS
                float deltaTime = Time.time - lastFrameTime;
                lastFrameTime = Time.time;

                fpsHistory.Enqueue(1f / deltaTime);
                if (fpsHistory.Count > 30) fpsHistory.Dequeue();

                float sum = 0f;
                foreach (var fps in fpsHistory) sum += fps;
                CurrentFPS = sum / fpsHistory.Count;

                //Invoke event
                OnFrameReceived?.Invoke(message.Frame);
            }
            else if (message?.Connected != null)
            {
                Debug.Log($"Welcome: {message.Connected.server_version} | Session: {message.Connected.session_id}");
            }
        }
        catch (Exception e)
        {
            Debug.LogError($"Failed to parse message: {e.Message}");
        }
    }

    private void OnWebSocketError(string error)
    {
        Debug.LogError($"WebSocket Error: {error}");
    }

    private void OnWebSocketClose(WebSocketCloseCode code)
    {
        bool wasConnected = IsConnected;
        IsConnected = false;
        Debug.Log($"Disconnected (code: {code})");

        if (wasConnected)
        {
            OnDisconnected?.Invoke();
        }

        if (autoReconnect && !isQuitting)
        {
            Debug.Log("Atempting to reconnect...");
        }
    }

    private async void SendClientConnect ()
    {
        var connectMsg = new ClientMessage
        {
            Connect = new ClientConnect
            {
                client_id = SystemInfo.deviceUniqueIdentifier,
                protocol_version = 256,
                capabilities = new ClientCapabilities
                {
                    device_name = "Meta Quest 3",
                    supports_hand_tracking = true,
                    supports_spatial_audio = true,
                    max_fps = 90
                }
            }
        };

        string json = JsonConvert.SerializeObject(connectMsg);
        await websocket.SendText(json);
        Debug.Log("Sent Client Capabilities");
    }

    async void OnApplicationQuit()
    {
        isQuitting = true;
        autoReconnect = false;
        if (websocket != null && websocket.State == WebSocketState.Open)
        {
            Debug.Log("Closing Websocket");
            await websocket.Close();
        }
    }

    [Serializable]
    public class ServerMessage
    {
        public ConnectedMessage Connected;
        public ARFrame Frame;
    }

    [Serializable]
    public class ConnectedMessage
    {
        public string server_version;
        public string session_id;
    }

    [Serializable]
    public class ARFrame
    {
        public ulong timestamp;
        public uint frame_id;
        public ushort protocol_version;
        public List<DetectedObject> objects = new List<DetectedObject>();
    }

    [Serializable]
    public class DetectedObject
    {
        public string @class;
        public float confidence;
        public BoundingBox bbox;
    }

    [Serializable]
    public class BoundingBox
    {
        public float x, y, width, height;
    }

    [Serializable]
    public class ClientMessage
    {
        public ClientConnect Connect;
    }

    [Serializable]
    public class ClientConnect
    {
        public string client_id;
        public ushort protocol_version;
        public ClientCapabilities capabilities;
    }

    [Serializable]
    public class ClientCapabilities
    {
        public string device_name;
        public bool supports_hand_tracking;
        public bool supports_spatial_audio;
        public uint max_fps;
    }
}