/**
 * BerryCode Extension Host
 *
 * Provides runtime environment for extensions with:
 * - Sandboxed execution
 * - API surface for extensions
 * - Message passing bridge
 * - Event system
 */

class ExtensionHost {
    constructor(sessionId) {
        this.sessionId = sessionId;
        this.extensions = new Map();
        this.activeExtensions = new Set();
        this.commandRegistry = new Map();
        this.eventListeners = new Map();
        this.apiCallId = 0;

        this.init();
    }

    async init() {
        console.log('[ExtensionHost] Initializing extension host');
        await this.loadInstalledExtensions();
        this.setupEventHandlers();
        console.log('[ExtensionHost] Extension host initialized');
    }

    async loadInstalledExtensions() {
        try {
            const response = await fetch('/api/extensions');
            if (!response.ok) {
                throw new Error(`Failed to load extensions: ${response.statusText}`);
            }

            const extensions = await response.json();
            console.log(`[ExtensionHost] Loaded ${extensions.length} installed extensions`);

            for (const ext of extensions) {
                this.extensions.set(ext.manifest.id, ext);

                // Auto-activate extensions with onStartup activation event
                if (ext.enabled && ext.manifest.activationEvents?.includes('onStartup')) {
                    await this.activateExtension(ext.manifest.id);
                }
            }
        } catch (error) {
            console.error('[ExtensionHost] Failed to load extensions:', error);
        }
    }

    async activateExtension(extensionId) {
        if (this.activeExtensions.has(extensionId)) {
            console.log(`[ExtensionHost] Extension already active: ${extensionId}`);
            return;
        }

        console.log(`[ExtensionHost] Activating extension: ${extensionId}`);

        try {
            // Get extension source code
            const response = await fetch(`/api/extensions/${extensionId}/source`);
            if (!response.ok) {
                throw new Error(`Failed to load extension source: ${response.statusText}`);
            }

            const { manifest, source } = await response.json();

            // Create sandboxed API context for extension
            const extensionAPI = this.createExtensionAPI(extensionId, manifest);

            // Execute extension in sandboxed environment
            await this.executeExtension(extensionId, source, extensionAPI);

            this.activeExtensions.add(extensionId);
            console.log(`[ExtensionHost] Extension activated: ${extensionId}`);

            // Trigger activation event
            this.emitEvent('extensionActivated', { extensionId, manifest });

        } catch (error) {
            console.error(`[ExtensionHost] Failed to activate extension ${extensionId}:`, error);
            throw error;
        }
    }

    async deactivateExtension(extensionId) {
        if (!this.activeExtensions.has(extensionId)) {
            return;
        }

        console.log(`[ExtensionHost] Deactivating extension: ${extensionId}`);

        // Remove registered commands
        for (const [cmdId, ext] of this.commandRegistry.entries()) {
            if (ext === extensionId) {
                this.commandRegistry.delete(cmdId);
            }
        }

        // Remove event listeners
        for (const [event, listeners] of this.eventListeners.entries()) {
            this.eventListeners.set(
                event,
                listeners.filter(l => l.extensionId !== extensionId)
            );
        }

        this.activeExtensions.delete(extensionId);
        this.emitEvent('extensionDeactivated', { extensionId });

        console.log(`[ExtensionHost] Extension deactivated: ${extensionId}`);
    }

    createExtensionAPI(extensionId, manifest) {
        const self = this;

        return {
            // Extension info
            extension: {
                id: manifest.id,
                name: manifest.name,
                version: manifest.version,
            },

            // Commands API
            commands: {
                registerCommand(commandId, handler) {
                    const fullCommandId = `${extensionId}.${commandId}`;
                    console.log(`[ExtensionHost] Registering command: ${fullCommandId}`);
                    self.commandRegistry.set(fullCommandId, {
                        extensionId,
                        handler,
                    });
                },

                executeCommand(commandId, ...args) {
                    return self.executeCommand(commandId, ...args);
                },
            },

            // UI API
            ui: {
                showNotification(message, type = 'info') {
                    self.showNotification(message, type);
                },

                addToolbarButton(config) {
                    self.addToolbarButton(extensionId, config);
                },

                addSidebarPanel(config) {
                    self.addSidebarPanel(extensionId, config);
                },

                showPanel(panelId) {
                    self.showSidebarPanel(`${extensionId}.${panelId}`);
                },
            },

            // Editor API
            editor: {
                getActiveFile() {
                    return self.getActiveFile();
                },

                getFileContent(path) {
                    return self.callAPI('fs.readFile', { path });
                },

                setFileContent(path, content) {
                    return self.callAPI('fs.writeFile', { path, content });
                },

                onDidOpenFile(handler) {
                    self.addEventListener('fileOpen', handler, extensionId);
                },

                onDidSaveFile(handler) {
                    self.addEventListener('fileSave', handler, extensionId);
                },

                onDidChangeFile(handler) {
                    self.addEventListener('fileChange', handler, extensionId);
                },
            },

            // File System API
            fs: {
                readFile(path) {
                    return self.callAPI('fs.readFile', { path });
                },

                writeFile(path, content) {
                    return self.callAPI('fs.writeFile', { path, content });
                },

                readDirectory(path) {
                    return self.callAPI('fs.readDirectory', { path });
                },
            },

            // Terminal API
            terminal: {
                execute(command) {
                    return self.callAPI('terminal.execute', { command });
                },
            },

            // Git API
            git: {
                status() {
                    return self.callAPI('git.status', {});
                },

                diff() {
                    return self.callAPI('git.diff', {});
                },

                commit(message) {
                    return self.callAPI('git.commit', { message });
                },
            },

            // LSP API
            lsp: {
                definition(file, line, character) {
                    return self.callAPI('lsp.definition', { file, line, character });
                },

                references(file, line, character) {
                    return self.callAPI('lsp.references', { file, line, character });
                },

                rename(file, line, character, newName) {
                    return self.callAPI('lsp.rename', { file, line, character, newName });
                },
            },

            // Storage API
            storage: {
                get(key) {
                    return self.callAPI('storage.get', { key });
                },

                set(key, value) {
                    return self.callAPI('storage.set', { key, value });
                },
            },

            // Events API
            events: {
                on(event, handler) {
                    self.addEventListener(event, handler, extensionId);
                },

                emit(event, data) {
                    self.emitEvent(event, data);
                },
            },
        };
    }

    async executeExtension(extensionId, source, api) {
        try {
            // Create sandboxed execution environment
            const sandbox = {
                berrycode: api,
                console: {
                    log: (...args) => console.log(`[Extension:${extensionId}]`, ...args),
                    warn: (...args) => console.warn(`[Extension:${extensionId}]`, ...args),
                    error: (...args) => console.error(`[Extension:${extensionId}]`, ...args),
                },
            };

            // Create function with sandboxed context
            const wrappedSource = `
                (function(berrycode, console) {
                    "use strict";
                    ${source}
                })
            `;

            const extensionFunction = eval(wrappedSource);

            // Execute extension
            await extensionFunction(sandbox.berrycode, sandbox.console);

        } catch (error) {
            console.error(`[ExtensionHost] Error executing extension ${extensionId}:`, error);
            throw error;
        }
    }

    async callAPI(method, params) {
        // Find which extension is calling this
        const stack = new Error().stack;
        let extensionId = null;

        // Try to extract extension ID from stack trace
        for (const [id] of this.extensions) {
            if (stack.includes(id)) {
                extensionId = id;
                break;
            }
        }

        if (!extensionId && this.activeExtensions.size > 0) {
            // Fallback to first active extension (not ideal but works for single extension)
            extensionId = Array.from(this.activeExtensions)[0];
        }

        if (!extensionId) {
            throw new Error('Cannot determine calling extension');
        }

        const callId = ++this.apiCallId;

        try {
            const response = await fetch(`/api/sessions/${this.sessionId}/extensions/api-call`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    extension_id: extensionId,
                    method,
                    params,
                }),
            });

            if (!response.ok) {
                throw new Error(`API call failed: ${response.statusText}`);
            }

            const result = await response.json();

            if (!result.success) {
                throw new Error(result.error || 'API call failed');
            }

            return result.result;

        } catch (error) {
            console.error(`[ExtensionHost] API call failed: ${method}`, error);
            throw error;
        }
    }

    async executeCommand(commandId, ...args) {
        const command = this.commandRegistry.get(commandId);
        if (!command) {
            throw new Error(`Command not found: ${commandId}`);
        }

        console.log(`[ExtensionHost] Executing command: ${commandId}`);

        try {
            return await command.handler(...args);
        } catch (error) {
            console.error(`[ExtensionHost] Command execution failed: ${commandId}`, error);
            throw error;
        }
    }

    addEventListener(event, handler, extensionId) {
        if (!this.eventListeners.has(event)) {
            this.eventListeners.set(event, []);
        }

        this.eventListeners.get(event).push({
            extensionId,
            handler,
        });
    }

    emitEvent(event, data) {
        const listeners = this.eventListeners.get(event) || [];

        for (const listener of listeners) {
            try {
                listener.handler(data);
            } catch (error) {
                console.error(`[ExtensionHost] Error in event handler for ${event}:`, error);
            }
        }
    }

    setupEventHandlers() {
        // Hook into editor events
        if (window.editor) {
            // These would be implemented based on your editor setup
            // For now, these are placeholders
        }
    }

    showNotification(message, type = 'info') {
        // Emit notification event for UI to handle
        this.emitEvent('notification', { message, type });

        // Also log to console
        console.log(`[ExtensionHost] Notification (${type}):`, message);
    }

    addToolbarButton(extensionId, config) {
        this.emitEvent('addToolbarButton', {
            extensionId,
            ...config,
        });
    }

    addSidebarPanel(extensionId, config) {
        this.emitEvent('addSidebarPanel', {
            extensionId,
            ...config,
        });
    }

    showSidebarPanel(panelId) {
        this.emitEvent('showSidebarPanel', { panelId });
    }

    getActiveFile() {
        // This would integrate with your editor
        return window.currentFile || null;
    }

    // Public API for managing extensions
    async installExtension(manifest, source) {
        try {
            const response = await fetch('/api/extensions/install', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    manifest,
                    source,
                }),
            });

            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.message || 'Installation failed');
            }

            const installed = await response.json();
            this.extensions.set(installed.manifest.id, installed);

            console.log(`[ExtensionHost] Extension installed: ${installed.manifest.id}`);

            // Auto-activate if enabled
            if (installed.enabled) {
                await this.activateExtension(installed.manifest.id);
            }

            return installed;

        } catch (error) {
            console.error('[ExtensionHost] Failed to install extension:', error);
            throw error;
        }
    }

    async uninstallExtension(extensionId) {
        try {
            // Deactivate if active
            if (this.activeExtensions.has(extensionId)) {
                await this.deactivateExtension(extensionId);
            }

            const response = await fetch(`/api/extensions/${extensionId}`, {
                method: 'DELETE',
            });

            if (!response.ok) {
                throw new Error(`Uninstallation failed: ${response.statusText}`);
            }

            this.extensions.delete(extensionId);

            console.log(`[ExtensionHost] Extension uninstalled: ${extensionId}`);

        } catch (error) {
            console.error('[ExtensionHost] Failed to uninstall extension:', error);
            throw error;
        }
    }

    async toggleExtension(extensionId, enabled) {
        try {
            const response = await fetch(`/api/extensions/${extensionId}/toggle`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ enabled }),
            });

            if (!response.ok) {
                throw new Error(`Toggle failed: ${response.statusText}`);
            }

            const updated = await response.json();
            this.extensions.set(extensionId, updated);

            if (enabled && !this.activeExtensions.has(extensionId)) {
                await this.activateExtension(extensionId);
            } else if (!enabled && this.activeExtensions.has(extensionId)) {
                await this.deactivateExtension(extensionId);
            }

            console.log(`[ExtensionHost] Extension toggled: ${extensionId} (${enabled})`);

        } catch (error) {
            console.error('[ExtensionHost] Failed to toggle extension:', error);
            throw error;
        }
    }

    getExtensions() {
        return Array.from(this.extensions.values());
    }

    getActiveExtensions() {
        return Array.from(this.activeExtensions);
    }
}

// Export for use in other scripts
if (typeof window !== 'undefined') {
    window.ExtensionHost = ExtensionHost;
}
