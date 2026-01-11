/**
 * Voice Input Manager
 * Supports both Web Speech API (browser native) and Whisper API (server-side)
 */

class VoiceInputManager {
    constructor() {
        this.isRecording = false;
        this.recognition = null;
        this.mediaRecorder = null;
        this.audioChunks = [];
        this.useWebSpeech = this.checkWebSpeechSupport();

        // Initialize Web Speech API if available
        if (this.useWebSpeech) {
            this.initializeWebSpeech();
        }

        console.log('VoiceInputManager initialized (Web Speech API:', this.useWebSpeech, ')');
    }

    /**
     * Check if Web Speech API is supported
     */
    checkWebSpeechSupport() {
        return 'webkitSpeechRecognition' in window || 'SpeechRecognition' in window;
    }

    /**
     * Initialize Web Speech API
     */
    initializeWebSpeech() {
        const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
        this.recognition = new SpeechRecognition();

        // Configuration
        this.recognition.continuous = true;
        this.recognition.interimResults = true;
        this.recognition.lang = 'ja-JP'; // Japanese by default, can be changed

        // Event handlers
        this.recognition.onstart = () => {
            console.log('Speech recognition started');
            this.updateUI(true);
        };

        this.recognition.onresult = (event) => {
            let interimTranscript = '';
            let finalTranscript = '';

            for (let i = event.resultIndex; i < event.results.length; i++) {
                const transcript = event.results[i][0].transcript;
                if (event.results[i].isFinal) {
                    finalTranscript += transcript + ' ';
                } else {
                    interimTranscript += transcript;
                }
            }

            // Update UI with transcription
            if (finalTranscript) {
                this.insertTranscription(finalTranscript.trim());
            }

            // Show interim results
            this.showInterimResults(interimTranscript);
        };

        this.recognition.onerror = (event) => {
            console.error('Speech recognition error:', event.error);
            this.showError('Voice recognition error: ' + event.error);
            this.stopRecording();
        };

        this.recognition.onend = () => {
            console.log('Speech recognition ended');
            if (this.isRecording) {
                // Restart if still in recording mode
                this.recognition.start();
            } else {
                this.updateUI(false);
            }
        };
    }

    /**
     * Initialize MediaRecorder for Whisper API fallback
     */
    async initializeMediaRecorder() {
        try {
            const stream = await navigator.mediaDevices.getUserMedia({ audio: true });

            // Use WebM Opus if supported, fallback to other formats
            const mimeType = MediaRecorder.isTypeSupported('audio/webm;codecs=opus')
                ? 'audio/webm;codecs=opus'
                : 'audio/webm';

            this.mediaRecorder = new MediaRecorder(stream, { mimeType });
            this.audioChunks = [];

            this.mediaRecorder.ondataavailable = (event) => {
                if (event.data.size > 0) {
                    this.audioChunks.push(event.data);
                }
            };

            this.mediaRecorder.onstop = async () => {
                const audioBlob = new Blob(this.audioChunks, { type: mimeType });
                await this.transcribeWithWhisper(audioBlob);

                // Stop all tracks
                stream.getTracks().forEach(track => track.stop());
            };

            console.log('MediaRecorder initialized');
        } catch (error) {
            console.error('Failed to initialize MediaRecorder:', error);
            throw error;
        }
    }

    /**
     * Start recording
     */
    async startRecording() {
        if (this.isRecording) {
            return;
        }

        this.isRecording = true;
        console.log('Starting voice input...');

        try {
            if (this.useWebSpeech) {
                // Use Web Speech API
                this.recognition.start();
            } else {
                // Use Whisper API with MediaRecorder
                await this.initializeMediaRecorder();
                this.mediaRecorder.start();
                this.updateUI(true);
                this.showStatus('Recording... Click stop when done.');
            }
        } catch (error) {
            console.error('Failed to start recording:', error);
            this.showError('Failed to start recording: ' + error.message);
            this.isRecording = false;
        }
    }

    /**
     * Stop recording
     */
    stopRecording() {
        if (!this.isRecording) {
            return;
        }

        this.isRecording = false;
        console.log('Stopping voice input...');

        if (this.useWebSpeech && this.recognition) {
            this.recognition.stop();
        } else if (this.mediaRecorder && this.mediaRecorder.state !== 'inactive') {
            this.mediaRecorder.stop();
        }

        this.updateUI(false);
        this.hideStatus();
    }

    /**
     * Toggle recording
     */
    toggleRecording() {
        if (this.isRecording) {
            this.stopRecording();
        } else {
            this.startRecording();
        }
    }

    /**
     * Transcribe audio using Whisper API
     */
    async transcribeWithWhisper(audioBlob) {
        this.showStatus('Transcribing...');

        try {
            const formData = new FormData();
            formData.append('file', audioBlob, 'audio.webm');

            const response = await fetch('/api/voice/transcribe', {
                method: 'POST',
                body: formData
            });

            if (!response.ok) {
                const errorData = await response.json();
                throw new Error(errorData.error || 'Transcription failed');
            }

            const data = await response.json();
            this.insertTranscription(data.text);
            this.showSuccess('Transcription complete!');
        } catch (error) {
            console.error('Whisper transcription error:', error);
            this.showError('Transcription failed: ' + error.message);
        } finally {
            this.hideStatus();
        }
    }

    /**
     * Insert transcription into chat input
     */
    insertTranscription(text) {
        const chatInput = document.getElementById('chat-input');
        if (chatInput) {
            const currentValue = chatInput.value;
            const newValue = currentValue ? currentValue + ' ' + text : text;
            chatInput.value = newValue;
            chatInput.focus();

            // Trigger input event for any listeners
            chatInput.dispatchEvent(new Event('input', { bubbles: true }));
        }
    }

    /**
     * Show interim results
     */
    showInterimResults(text) {
        const interimDisplay = document.getElementById('voice-interim');
        if (interimDisplay && text) {
            interimDisplay.textContent = text;
            interimDisplay.style.display = 'block';
        } else if (interimDisplay) {
            interimDisplay.style.display = 'none';
        }
    }

    /**
     * Update UI based on recording state
     */
    updateUI(isRecording) {
        const voiceBtn = document.getElementById('voice-btn');
        const voiceIcon = document.getElementById('voice-icon');

        if (voiceBtn) {
            if (isRecording) {
                voiceBtn.classList.add('recording');
                voiceBtn.title = 'Stop recording (Ctrl+M)';
            } else {
                voiceBtn.classList.remove('recording');
                voiceBtn.title = 'Start voice input (Ctrl+M)';
            }
        }

        if (voiceIcon) {
            voiceIcon.textContent = isRecording ? '⏹' : '🎤';
        }
    }

    /**
     * Show status message
     */
    showStatus(message) {
        const statusEl = document.getElementById('voice-status');
        if (statusEl) {
            statusEl.textContent = message;
            statusEl.style.display = 'block';
            statusEl.className = 'voice-status';
        }
    }

    /**
     * Show error message
     */
    showError(message) {
        const statusEl = document.getElementById('voice-status');
        if (statusEl) {
            statusEl.textContent = message;
            statusEl.style.display = 'block';
            statusEl.className = 'voice-status error';
        }
        setTimeout(() => this.hideStatus(), 5000);
    }

    /**
     * Show success message
     */
    showSuccess(message) {
        const statusEl = document.getElementById('voice-status');
        if (statusEl) {
            statusEl.textContent = message;
            statusEl.style.display = 'block';
            statusEl.className = 'voice-status success';
        }
        setTimeout(() => this.hideStatus(), 3000);
    }

    /**
     * Hide status message
     */
    hideStatus() {
        const statusEl = document.getElementById('voice-status');
        if (statusEl) {
            statusEl.style.display = 'none';
        }
    }

    /**
     * Change recognition language
     */
    setLanguage(lang) {
        if (this.recognition) {
            this.recognition.lang = lang;
            console.log('Recognition language set to:', lang);
        }
    }
}

// Initialize voice input manager globally
let voiceInputManager = null;

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    voiceInputManager = new VoiceInputManager();

    // Attach to voice button
    const voiceBtn = document.getElementById('voice-btn');
    if (voiceBtn) {
        voiceBtn.addEventListener('click', () => {
            voiceInputManager.toggleRecording();
        });
    }

    // Keyboard shortcut: Ctrl+M
    // Export for external use
    window.voiceInputManager = voiceInputManager;

    document.addEventListener('keydown', (event) => {
        if (event.ctrlKey && event.key === 'm') {
            event.preventDefault();
            voiceInputManager.toggleRecording();
        }
    });

    // Language selector (if exists)
    const langSelector = document.getElementById('voice-language');
    if (langSelector) {
        langSelector.addEventListener('change', (event) => {
            voiceInputManager.setLanguage(event.target.value);
        });
    }
});
