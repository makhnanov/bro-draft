import { ref, readonly } from 'vue';

// Storage keys
const AUDIO_DEVICE_STORAGE_KEY = 'selected-audio-device';
const LISTENING_ENABLED_STORAGE_KEY = 'microphone-listening-enabled';

// Mute detection settings (based on detector.py)
const THRESHOLD = 0.00005; // Very low threshold for silence detection
const SILENCE_LIMIT = 0.3; // Seconds of silence before considering muted

// State
const audioDevices = ref<MediaDeviceInfo[]>([]);
const selectedDeviceId = ref<string>('');
const isListening = ref(false);
const audioLevel = ref(0);
const isMuted = ref(false);

// Private audio context variables
let audioContext: AudioContext | null = null;
let analyser: AnalyserNode | null = null;
let microphone: MediaStreamAudioSourceNode | null = null;
let mediaStream: MediaStream | null = null;
let animationFrameId: number | null = null;
let lastSoundTime = Date.now();

// Callbacks for state changes
type MuteChangeCallback = (muted: boolean) => void;
const muteChangeCallbacks: MuteChangeCallback[] = [];

/**
 * Load available audio input devices
 */
async function loadAudioDevices(): Promise<void> {
    try {
        console.log('[MicrophoneService] Loading audio devices...');

        // Request permission first to get full device list with labels
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        console.log('[MicrophoneService] Got permission');

        // Stop the temporary stream
        stream.getTracks().forEach(track => track.stop());

        // Now enumerate devices - we'll get full labels
        const devices = await navigator.mediaDevices.enumerateDevices();
        const audioInputs = devices.filter(device => device.kind === 'audioinput');
        console.log(`[MicrophoneService] Found ${audioInputs.length} audio input devices`);

        audioDevices.value = audioInputs;

        // Load saved device or select first one
        const savedDeviceId = localStorage.getItem(AUDIO_DEVICE_STORAGE_KEY);
        if (savedDeviceId && audioDevices.value.some(d => d.deviceId === savedDeviceId)) {
            selectedDeviceId.value = savedDeviceId;
            console.log('[MicrophoneService] Loaded saved device:', savedDeviceId);
        } else if (audioDevices.value.length > 0) {
            selectedDeviceId.value = audioDevices.value[0].deviceId;
            console.log('[MicrophoneService] Selected default device:', selectedDeviceId.value);
        }
    } catch (error) {
        console.error('[MicrophoneService] Failed to enumerate audio devices:', error);
        // If permission denied, still try to get basic device list
        try {
            const devices = await navigator.mediaDevices.enumerateDevices();
            audioDevices.value = devices.filter(device => device.kind === 'audioinput');
        } catch (e) {
            console.error('[MicrophoneService] Failed to get device list:', e);
        }
    }
}

/**
 * Start listening to the microphone
 */
async function startListening(): Promise<void> {
    if (!selectedDeviceId.value) {
        console.warn('[MicrophoneService] No device selected');
        return;
    }

    if (isListening.value) {
        console.warn('[MicrophoneService] Already listening');
        return;
    }

    try {
        console.log('[MicrophoneService] Starting listening with device:', selectedDeviceId.value);

        // Request microphone access
        mediaStream = await navigator.mediaDevices.getUserMedia({
            audio: {
                deviceId: selectedDeviceId.value ? { exact: selectedDeviceId.value } : undefined
            }
        });

        // Create audio context and analyser
        audioContext = new AudioContext();
        analyser = audioContext.createAnalyser();
        analyser.fftSize = 256;
        analyser.smoothingTimeConstant = 0.8;

        microphone = audioContext.createMediaStreamSource(mediaStream);
        microphone.connect(analyser);

        isListening.value = true;
        isMuted.value = false;
        lastSoundTime = Date.now();

        // Save listening state
        localStorage.setItem(LISTENING_ENABLED_STORAGE_KEY, 'true');

        console.log('[MicrophoneService] Started listening');
        updateAudioLevel();
    } catch (error) {
        console.error('[MicrophoneService] Failed to start listening:', error);
        throw error;
    }
}

/**
 * Stop listening to the microphone
 */
function stopListening(): void {
    console.log('[MicrophoneService] Stopping listening');

    if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId);
        animationFrameId = null;
    }

    if (microphone) {
        microphone.disconnect();
        microphone = null;
    }

    if (analyser) {
        analyser = null;
    }

    if (audioContext) {
        audioContext.close();
        audioContext = null;
    }

    if (mediaStream) {
        mediaStream.getTracks().forEach(track => track.stop());
        mediaStream = null;
    }

    isListening.value = false;
    audioLevel.value = 0;
    isMuted.value = false;

    // Save listening state
    localStorage.setItem(LISTENING_ENABLED_STORAGE_KEY, 'false');

    console.log('[MicrophoneService] Stopped listening');
}

/**
 * Toggle listening on/off
 */
async function toggleListening(): Promise<void> {
    if (isListening.value) {
        stopListening();
    } else {
        await startListening();
    }
}

/**
 * Update audio level and check for mute state
 * Based on detector.py logic
 */
function updateAudioLevel(): void {
    if (!analyser || !isListening.value) return;

    const dataArray = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteFrequencyData(dataArray);

    // Calculate average volume (RMS-like)
    const average = dataArray.reduce((a, b) => a + b) / dataArray.length;
    const level = Math.min(100, (average / 255) * 100 * 2); // Scale to 0-100
    audioLevel.value = level;

    // Normalize level to a 0-1 range for threshold comparison
    const normalizedLevel = average / 255;

    // Check mute status (similar to detector.py)
    if (normalizedLevel > THRESHOLD) {
        // Sound detected
        lastSoundTime = Date.now();
        if (isMuted.value) {
            isMuted.value = false;
            console.log('[MicrophoneService] Microphone unmuted (Live)');
            notifyMuteChange(false);
        }
    } else {
        // Silence detected
        const silenceTime = (Date.now() - lastSoundTime) / 1000;
        if (silenceTime >= SILENCE_LIMIT && !isMuted.value) {
            isMuted.value = true;
            console.log('[MicrophoneService] Microphone muted (Hardware Mute)');
            notifyMuteChange(true);
        }
    }

    animationFrameId = requestAnimationFrame(updateAudioLevel);
}

/**
 * Notify all registered callbacks about mute state change
 */
function notifyMuteChange(muted: boolean): void {
    muteChangeCallbacks.forEach(callback => {
        try {
            callback(muted);
        } catch (error) {
            console.error('[MicrophoneService] Error in mute change callback:', error);
        }
    });
}

/**
 * Register a callback for mute state changes
 */
function onMuteChange(callback: MuteChangeCallback): () => void {
    muteChangeCallbacks.push(callback);
    // Return unsubscribe function
    return () => {
        const index = muteChangeCallbacks.indexOf(callback);
        if (index > -1) {
            muteChangeCallbacks.splice(index, 1);
        }
    };
}

/**
 * Set the selected audio device
 */
function setSelectedDevice(deviceId: string): void {
    if (selectedDeviceId.value === deviceId) return;

    console.log('[MicrophoneService] Setting device:', deviceId);
    selectedDeviceId.value = deviceId;
    localStorage.setItem(AUDIO_DEVICE_STORAGE_KEY, deviceId);

    // Restart listening if currently active
    if (isListening.value) {
        stopListening();
        setTimeout(() => startListening(), 100);
    }
}

/**
 * Initialize the service and auto-start if needed
 */
async function initialize(): Promise<void> {
    console.log('[MicrophoneService] Initializing...');
    await loadAudioDevices();

    // Auto-start listening if it was enabled before
    const wasListening = localStorage.getItem(LISTENING_ENABLED_STORAGE_KEY) === 'true';
    if (wasListening && selectedDeviceId.value) {
        console.log('[MicrophoneService] Auto-starting listening...');
        try {
            await startListening();
        } catch (error) {
            console.error('[MicrophoneService] Failed to auto-start listening:', error);
        }
    }
}

/**
 * Get device name with encoding fix
 */
function getDeviceName(device: MediaDeviceInfo): string {
    if (!device.label) {
        return `Устройство ${device.deviceId.substring(0, 8)}`;
    }
    return device.label;
}

// Export the service
export const microphoneService = {
    // State (readonly for external use)
    audioDevices: readonly(audioDevices),
    selectedDeviceId: readonly(selectedDeviceId),
    isListening: readonly(isListening),
    audioLevel: readonly(audioLevel),
    isMuted: readonly(isMuted),

    // Methods
    initialize,
    loadAudioDevices,
    startListening,
    stopListening,
    toggleListening,
    setSelectedDevice,
    onMuteChange,
    getDeviceName,
};

export default microphoneService;
