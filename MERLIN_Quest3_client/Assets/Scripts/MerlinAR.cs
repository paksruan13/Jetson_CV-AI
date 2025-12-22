using UnityEngine;
using TMPro;

//This will handle AR Frame rendering and UI Display
public class MerlinAR : MonoBehaviour
{
    [Header("References")]
    [SerializeField] private ARBridge arBridge;
    [SerializeField] private TextMeshProUGUI statusText;
    [SerializeField] private TextMeshProUGUI fpsText;

    [Header("AR Objects")]
    [SerializeField] private GameObject testCube;

    private int totalFrames = 0;

    void Start()
    {
        if (arBridge == null)
        {
            arBridge = FindFirstObjectByType<ARBridge>();
        }

        // Listen to events from ARBidge.cs
        arBridge.OnConnected += OnARBridgeConnected;
        arBridge.OnDisconnected += OnARBridgeDisconnected;
        arBridge.OnFrameReceived += OnARFrameReceived;

        if (testCube != null)
        {
            testCube.SetActive(false);
            Debug.Log("Test Cube set to inactive");
        }

        UpdateStatus("Connecting to MERLIN...");
    }

    private void OnARBridgeConnected()
    {
        UpdateStatus("Connected to MERLIN");

        //show test cube for now
        if (testCube != null)
        {
            testCube.SetActive(true);
            testCube.transform.position = new Vector3(0f, 1.5f, 2f);
            testCube.transform.rotation = Quaternion.identity;
            Debug.Log("Test Cube activated");
        }
    }

    private void OnARBridgeDisconnected()
    {
        UpdateStatus("Disconnected from MERLIN");
        if (testCube != null)
        {
            testCube.SetActive(false);
        }
    }

    private void OnARFrameReceived(ARBridge.ARFrame frame)
    {
        totalFrames++;

        if (fpsText != null) //Update FPS display here
        {
            fpsText.text = $"Frame #{frame.frame_id} | {arBridge.CurrentFPS:F1} FPS";
        }

        // Animate test cube based on frame data
        if (testCube != null)
        {
            // Rotate cube based on frame ID for now
            float rotation = frame.frame_id % 360;
            testCube.transform.rotation = Quaternion.Euler(rotation, rotation * 0.5f, 0f);
        }
    }

    private void UpdateStatus(string message)
    {
        Debug.Log(message);
        if (statusText != null)
        {
            statusText.text = message;
        }
    }

    void OnDestroy()
    {
        if (arBridge != null) //Unsubscribe to all events
        {
            arBridge.OnConnected -= OnARBridgeConnected;
            arBridge.OnDisconnected -= OnARBridgeDisconnected;
            arBridge.OnFrameReceived -= OnARFrameReceived;
        }
    }
}