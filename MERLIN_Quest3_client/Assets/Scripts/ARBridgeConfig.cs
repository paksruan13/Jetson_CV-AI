using UnityEngine;
[CreateAssetMenu(fileName = "ARBridgeConfig", menuName = "Configs/ARBridgeConfig")]
public class ARBridgeConfig : ScriptableObject
{
    [Header("Connection Settings")]
    [Tooltip("Jetson AR Bridge IP Address")]
    public string jetsonIp = "";

    [Tooltip("Jetson AR Bridge Port Number")]
    public int jetsonPort = 8765;

    [Tooltip("Reconnection delay in seconds")]
    public float reconnectDelay = 2f;

    [Tooltip("Auto-Reconnect on disconnection")]
    public bool autoReconnect = true;

    [Header("Performance")]
    [Tooltip("Target FPS for AR Streaming")]
    public int targetFPS = 30;
}