/**
 * Model Settings Manager
 * Manages AI model configuration for different task types
 */

class ModelSettingsManager {
    constructor() {
        this.currentSessionId = null;
        this.settings = {};
        this.availableModels = [];

        // Default models for each task type (Latest Generation)
        this.defaults = {
            'design': 'gpt-5.1-high',        // 総合知能 No.1
            'implementation': 'gpt-5.1-high', // 実装力 No.1
            'review': 'claude-4.5-sonnet',    // 推論・知識 No.1
            'test': 'grok-4-fast',            // コスパ最強
            'debug': 'gemini-2.5-flash-lite'  // 速度 No.1
        };
    }

    /**
     * Initialize with session ID
     */
    async init(sessionId) {
        this.currentSessionId = sessionId;
        await this.loadAvailableModels();
        await this.loadSettings();
    }

    /**
     * Load available AI models from API
     */
    async loadAvailableModels() {
        try {
            const response = await fetch('/api/models/list');
            if (!response.ok) {
                throw new Error(`Failed to load models: ${response.statusText}`);
            }
            this.availableModels = await response.json();
            console.log('Loaded available models:', this.availableModels);
        } catch (error) {
            console.error('Error loading available models:', error);
            // Use empty array on error
            this.availableModels = [];
        }
    }

    /**
     * Load current settings from API
     */
    async loadSettings() {
        if (!this.currentSessionId) {
            console.warn('No session ID set');
            return;
        }

        try {
            const response = await fetch(`/api/model-settings/${this.currentSessionId}`);
            if (!response.ok) {
                throw new Error(`Failed to load settings: ${response.statusText}`);
            }
            const data = await response.json();
            this.settings = data.settings || {};
            console.log('Loaded model settings:', this.settings);
        } catch (error) {
            console.error('Error loading model settings:', error);
            // Use defaults on error
            this.settings = { ...this.defaults };
        }
    }

    /**
     * Save settings to API
     */
    async saveSettings(settings) {
        if (!this.currentSessionId) {
            console.warn('No session ID set');
            return false;
        }

        try {
            const response = await fetch(`/api/model-settings/${this.currentSessionId}`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ settings }),
            });

            if (!response.ok) {
                throw new Error(`Failed to save settings: ${response.statusText}`);
            }

            const data = await response.json();
            this.settings = data.settings || {};
            console.log('Saved model settings:', this.settings);
            return true;
        } catch (error) {
            console.error('Error saving model settings:', error);
            return false;
        }
    }

    /**
     * Get model for a specific task type
     */
    getModelForTask(taskType) {
        return this.settings[taskType] || this.defaults[taskType] || 'gpt-4o';
    }

    /**
     * Get model information
     */
    getModelInfo(modelName) {
        return this.availableModels.find(m => m.name === modelName);
    }

    /**
     * Get default settings
     */
    getDefaultSettings() {
        return { ...this.defaults };
    }

    /**
     * Reset to default settings
     */
    async resetToDefaults() {
        this.settings = { ...this.defaults };
        return await this.saveSettings(this.settings);
    }

    /**
     * Group models by provider
     */
    getModelsByProvider() {
        const grouped = {};
        for (const model of this.availableModels) {
            if (!grouped[model.provider]) {
                grouped[model.provider] = [];
            }
            grouped[model.provider].push(model);
        }
        return grouped;
    }

    /**
     * Format cost display
     */
    formatCost(inputCost, outputCost) {
        return `In $${inputCost.toFixed(2)} / Out $${outputCost.toFixed(2)} per 1M tokens`;
    }

    /**
     * Format context window
     */
    formatContextWindow(tokens) {
        if (tokens >= 1000000) {
            return `${(tokens / 1000000).toFixed(1)}M tokens`;
        } else if (tokens >= 1000) {
            return `${(tokens / 1000).toFixed(0)}K tokens`;
        }
        return `${tokens} tokens`;
    }

    /**
     * Check if model is high cost (>$10/M output)
     */
    isHighCostModel(modelName) {
        const model = this.getModelInfo(modelName);
        return model && model.output_cost > 10.0;
    }
}

// Global instance
let modelSettingsManager = null;

/**
 * Initialize model settings manager
 */
function initModelSettings(sessionId) {
    modelSettingsManager = new ModelSettingsManager();
    return modelSettingsManager.init(sessionId);
}

/**
 * Get current model settings manager instance
 */
function getModelSettingsManager() {
    return modelSettingsManager;
}

/**
 * Populate model select dropdown
 */
function populateModelSelect(selectElement, currentValue) {
    if (!modelSettingsManager) {
        console.warn('Model settings manager not initialized');
        return;
    }

    const modelsByProvider = modelSettingsManager.getModelsByProvider();

    // Clear existing options
    selectElement.innerHTML = '';

    // Add options grouped by provider
    for (const [provider, models] of Object.entries(modelsByProvider)) {
        const optgroup = document.createElement('optgroup');
        optgroup.label = provider;

        for (const model of models) {
            const option = document.createElement('option');
            option.value = model.name;
            option.textContent = model.display_name;
            if (model.name === currentValue) {
                option.selected = true;
            }
            optgroup.appendChild(option);
        }

        selectElement.appendChild(optgroup);
    }
}

/**
 * Update model info display
 */
function updateModelInfo(infoElement, modelName) {
    if (!modelSettingsManager) {
        console.warn('Model settings manager not initialized');
        return;
    }

    const model = modelSettingsManager.getModelInfo(modelName);
    if (!model) {
        infoElement.innerHTML = '<span style="color: #666;">Model information not available</span>';
        return;
    }

    const contextWindow = modelSettingsManager.formatContextWindow(model.max_context_tokens);
    const cost = modelSettingsManager.formatCost(model.input_cost, model.output_cost);
    const vision = model.supports_vision ? '✓' : '✗';
    const isHighCost = modelSettingsManager.isHighCostModel(modelName);

    infoElement.innerHTML = `
        <div style="font-size: 0.85em; color: #888; margin-top: 4px;">
            <div>Context: ${contextWindow}</div>
            <div>Cost: ${cost}${isHighCost ? ' <span style="color: #f5a623;">⚠ High Cost</span>' : ''}</div>
            <div>Vision: ${vision}</div>
        </div>
    `;
}
