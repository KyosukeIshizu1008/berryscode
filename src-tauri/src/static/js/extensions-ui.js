/**
 * BerryCode Extensions UI
 *
 * Provides the marketplace and management UI for extensions
 */

class ExtensionsUI {
    constructor(extensionHost) {
        this.extensionHost = extensionHost;
        this.currentView = 'marketplace'; // marketplace, installed, create
        this.init();
    }

    init() {
        this.setupEventListeners();
        this.render();
    }

    setupEventListeners() {
        // Listen for extension events
        this.extensionHost.addEventListener('extensionActivated', (data) => {
            this.showNotification(`Extension activated: ${data.manifest.name}`, 'success');
            this.render();
        });

        this.extensionHost.addEventListener('extensionDeactivated', (data) => {
            this.showNotification(`Extension deactivated: ${data.extensionId}`, 'info');
            this.render();
        });

        this.extensionHost.addEventListener('notification', (data) => {
            this.showNotification(data.message, data.type);
        });

        this.extensionHost.addEventListener('addToolbarButton', (data) => {
            this.addToolbarButton(data);
        });

        this.extensionHost.addEventListener('addSidebarPanel', (data) => {
            this.addSidebarPanel(data);
        });
    }

    render() {
        const container = document.getElementById('extensions-panel');
        if (!container) {
            console.warn('[ExtensionsUI] Extensions panel not found');
            return;
        }

        container.innerHTML = `
            <div class="extensions-container">
                <div class="extensions-header">
                    <div class="extensions-tabs">
                        <button class="extensions-tab ${this.currentView === 'marketplace' ? 'active' : ''}"
                                onclick="extensionsUI.switchView('marketplace')">
                            Marketplace
                        </button>
                        <button class="extensions-tab ${this.currentView === 'installed' ? 'active' : ''}"
                                onclick="extensionsUI.switchView('installed')">
                            Installed
                        </button>
                        <button class="extensions-tab ${this.currentView === 'create' ? 'active' : ''}"
                                onclick="extensionsUI.switchView('create')">
                            Create Extension
                        </button>
                    </div>
                    <div class="extensions-actions">
                        <button class="btn-icon" onclick="extensionsUI.refreshExtensions()" title="Refresh">
                            <svg viewBox="0 0 24 24" width="16" height="16">
                                <path fill="currentColor" d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/>
                            </svg>
                        </button>
                    </div>
                </div>
                <div class="extensions-content">
                    ${this.renderContent()}
                </div>
            </div>
        `;
    }

    renderContent() {
        switch (this.currentView) {
            case 'marketplace':
                return this.renderMarketplace();
            case 'installed':
                return this.renderInstalled();
            case 'create':
                return this.renderCreate();
            default:
                return '<div>Unknown view</div>';
        }
    }

    renderMarketplace() {
        // Sample marketplace extensions
        const marketplaceExtensions = [
            {
                id: 'prettier-format',
                name: 'Prettier Code Formatter',
                description: 'Format your code with Prettier',
                author: 'BerryCode Team',
                version: '1.0.0',
                icon: '✨',
                categories: ['productivity', 'editor'],
            },
            {
                id: 'git-blame',
                name: 'Git Blame Viewer',
                description: 'Show git blame information inline',
                author: 'BerryCode Team',
                version: '1.0.0',
                icon: '📝',
                categories: ['git'],
            },
            {
                id: 'todo-highlighter',
                name: 'TODO Highlighter',
                description: 'Highlight TODO, FIXME, and NOTE comments',
                author: 'Community',
                version: '1.2.0',
                icon: '📌',
                categories: ['productivity'],
            },
        ];

        const installed = this.extensionHost.getExtensions().map(e => e.manifest.id);

        return `
            <div class="extensions-marketplace">
                <div class="extensions-search">
                    <input type="text"
                           class="extensions-search-input"
                           placeholder="Search extensions..."
                           oninput="extensionsUI.searchExtensions(this.value)">
                </div>
                <div class="extensions-grid" id="extensions-marketplace-grid">
                    ${marketplaceExtensions.map(ext => `
                        <div class="extension-card">
                            <div class="extension-icon">${ext.icon || '🧩'}</div>
                            <div class="extension-info">
                                <h3 class="extension-name">${ext.name}</h3>
                                <p class="extension-description">${ext.description}</p>
                                <div class="extension-meta">
                                    <span class="extension-author">${ext.author}</span>
                                    <span class="extension-version">v${ext.version}</span>
                                </div>
                                <div class="extension-categories">
                                    ${ext.categories.map(cat => `
                                        <span class="extension-category">${cat}</span>
                                    `).join('')}
                                </div>
                            </div>
                            <div class="extension-actions">
                                ${installed.includes(ext.id)
                                    ? '<button class="btn-secondary" disabled>Installed</button>'
                                    : `<button class="btn-primary" onclick="extensionsUI.installFromMarketplace('${ext.id}')">Install</button>`
                                }
                            </div>
                        </div>
                    `).join('')}
                </div>
            </div>
        `;
    }

    renderInstalled() {
        const extensions = this.extensionHost.getExtensions();
        const active = this.extensionHost.getActiveExtensions();

        if (extensions.length === 0) {
            return `
                <div class="extensions-empty">
                    <svg viewBox="0 0 24 24" width="64" height="64">
                        <path fill="currentColor" d="M20.5 11H19V7c0-1.1-.9-2-2-2h-4V3.5C13 2.12 11.88 1 10.5 1S8 2.12 8 3.5V5H4c-1.1 0-1.99.9-1.99 2v3.8H3.5c1.49 0 2.7 1.21 2.7 2.7s-1.21 2.7-2.7 2.7H2V20c0 1.1.9 2 2 2h3.8v-1.5c0-1.49 1.21-2.7 2.7-2.7 1.49 0 2.7 1.21 2.7 2.7V22H17c1.1 0 2-.9 2-2v-4h1.5c1.38 0 2.5-1.12 2.5-2.5S21.88 11 20.5 11z"/>
                    </svg>
                    <h3>No Extensions Installed</h3>
                    <p>Browse the marketplace to find extensions</p>
                    <button class="btn-primary" onclick="extensionsUI.switchView('marketplace')">
                        Browse Marketplace
                    </button>
                </div>
            `;
        }

        return `
            <div class="extensions-installed">
                <div class="extensions-list">
                    ${extensions.map(ext => `
                        <div class="extension-item ${ext.enabled ? 'enabled' : 'disabled'}">
                            <div class="extension-item-header">
                                <div class="extension-item-info">
                                    <div class="extension-item-icon">${ext.manifest.icon || '🧩'}</div>
                                    <div class="extension-item-details">
                                        <h4 class="extension-item-name">
                                            ${ext.manifest.name}
                                            ${active.includes(ext.manifest.id) ? '<span class="extension-status active">Active</span>' : ''}
                                        </h4>
                                        <p class="extension-item-description">${ext.manifest.description || 'No description'}</p>
                                        <div class="extension-item-meta">
                                            <span class="extension-item-author">${ext.manifest.author || 'Unknown'}</span>
                                            <span class="extension-item-version">v${ext.manifest.version}</span>
                                        </div>
                                    </div>
                                </div>
                                <div class="extension-item-actions">
                                    <label class="toggle-switch">
                                        <input type="checkbox"
                                               ${ext.enabled ? 'checked' : ''}
                                               onchange="extensionsUI.toggleExtension('${ext.manifest.id}', this.checked)">
                                        <span class="toggle-slider"></span>
                                    </label>
                                    <button class="btn-icon btn-danger"
                                            onclick="extensionsUI.uninstallExtension('${ext.manifest.id}')"
                                            title="Uninstall">
                                        <svg viewBox="0 0 24 24" width="16" height="16">
                                            <path fill="currentColor" d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                                        </svg>
                                    </button>
                                </div>
                            </div>
                            ${this.renderExtensionCommands(ext)}
                        </div>
                    `).join('')}
                </div>
            </div>
        `;
    }

    renderExtensionCommands(ext) {
        if (!ext.manifest.contributes?.commands || ext.manifest.contributes.commands.length === 0) {
            return '';
        }

        return `
            <div class="extension-item-commands">
                <h5>Commands:</h5>
                <div class="extension-commands-list">
                    ${ext.manifest.contributes.commands.map(cmd => `
                        <div class="extension-command">
                            <span class="extension-command-title">${cmd.title}</span>
                            ${cmd.keybinding ? `<kbd>${cmd.keybinding}</kbd>` : ''}
                        </div>
                    `).join('')}
                </div>
            </div>
        `;
    }

    renderCreate() {
        return `
            <div class="extensions-create">
                <div class="extensions-create-form">
                    <h3>Create New Extension</h3>
                    <form id="create-extension-form" onsubmit="extensionsUI.createExtension(event)">
                        <div class="form-group">
                            <label>Extension ID</label>
                            <input type="text" id="ext-id" name="id" required
                                   pattern="[a-z0-9-]+"
                                   placeholder="my-extension"
                                   class="form-control">
                            <small>Lowercase letters, numbers, and hyphens only</small>
                        </div>

                        <div class="form-group">
                            <label>Name</label>
                            <input type="text" id="ext-name" name="name" required
                                   placeholder="My Extension"
                                   class="form-control">
                        </div>

                        <div class="form-group">
                            <label>Version</label>
                            <input type="text" id="ext-version" name="version"
                                   value="1.0.0" required
                                   pattern="\\d+\\.\\d+\\.\\d+"
                                   class="form-control">
                            <small>Semantic versioning (x.y.z)</small>
                        </div>

                        <div class="form-group">
                            <label>Description</label>
                            <textarea id="ext-description" name="description"
                                      placeholder="What does your extension do?"
                                      class="form-control" rows="3"></textarea>
                        </div>

                        <div class="form-group">
                            <label>Author</label>
                            <input type="text" id="ext-author" name="author"
                                   placeholder="Your name"
                                   class="form-control">
                        </div>

                        <div class="form-group">
                            <label>Extension Code</label>
                            <textarea id="ext-source" name="source" required
                                      class="form-control code-editor" rows="15"
                                      placeholder="// Extension code here&#x0a;berrycode.commands.registerCommand('myCommand', () => {&#x0a;    berrycode.ui.showNotification('Hello from my extension!');&#x0a;});"></textarea>
                        </div>

                        <div class="form-group">
                            <label>Permissions</label>
                            <div class="form-checkboxes">
                                <label><input type="checkbox" name="perm-fs-read" value="fileSystem:read"> File System (Read)</label>
                                <label><input type="checkbox" name="perm-fs-write" value="fileSystem:write"> File System (Write)</label>
                                <label><input type="checkbox" name="perm-terminal" value="terminal:execute"> Terminal Execute</label>
                                <label><input type="checkbox" name="perm-git-read" value="git:read"> Git (Read)</label>
                                <label><input type="checkbox" name="perm-git-write" value="git:write"> Git (Write)</label>
                                <label><input type="checkbox" name="perm-lsp-read" value="lsp:read"> LSP (Read)</label>
                                <label><input type="checkbox" name="perm-lsp-write" value="lsp:write"> LSP (Write)</label>
                                <label><input type="checkbox" name="perm-storage" value="storage:local"> Local Storage</label>
                            </div>
                        </div>

                        <div class="form-actions">
                            <button type="submit" class="btn-primary">Create & Install</button>
                            <button type="button" class="btn-secondary" onclick="extensionsUI.loadTemplate()">Load Template</button>
                        </div>
                    </form>
                </div>

                <div class="extensions-create-help">
                    <h4>Extension API Reference</h4>
                    <div class="api-reference">
                        <details>
                            <summary>Commands API</summary>
                            <pre><code>berrycode.commands.registerCommand(id, handler)
berrycode.commands.executeCommand(id, ...args)</code></pre>
                        </details>

                        <details>
                            <summary>UI API</summary>
                            <pre><code>berrycode.ui.showNotification(message, type)
berrycode.ui.addToolbarButton(config)
berrycode.ui.addSidebarPanel(config)</code></pre>
                        </details>

                        <details>
                            <summary>Editor API</summary>
                            <pre><code>berrycode.editor.getActiveFile()
berrycode.editor.getFileContent(path)
berrycode.editor.setFileContent(path, content)
berrycode.editor.onDidOpenFile(handler)
berrycode.editor.onDidSaveFile(handler)</code></pre>
                        </details>

                        <details>
                            <summary>Storage API</summary>
                            <pre><code>await berrycode.storage.get(key)
await berrycode.storage.set(key, value)</code></pre>
                        </details>
                    </div>
                </div>
            </div>
        `;
    }

    switchView(view) {
        this.currentView = view;
        this.render();
    }

    async refreshExtensions() {
        await this.extensionHost.loadInstalledExtensions();
        this.render();
        this.showNotification('Extensions refreshed', 'success');
    }

    async toggleExtension(extensionId, enabled) {
        try {
            await this.extensionHost.toggleExtension(extensionId, enabled);
            this.showNotification(
                `Extension ${enabled ? 'enabled' : 'disabled'}`,
                'success'
            );
        } catch (error) {
            this.showNotification(`Failed to toggle extension: ${error.message}`, 'error');
            this.render();
        }
    }

    async uninstallExtension(extensionId) {
        if (!confirm('Are you sure you want to uninstall this extension?')) {
            return;
        }

        try {
            await this.extensionHost.uninstallExtension(extensionId);
            this.render();
            this.showNotification('Extension uninstalled', 'success');
        } catch (error) {
            this.showNotification(`Failed to uninstall extension: ${error.message}`, 'error');
        }
    }

    async createExtension(event) {
        event.preventDefault();
        const form = event.target;
        const formData = new FormData(form);

        // Collect permissions
        const permissions = [];
        form.querySelectorAll('input[type="checkbox"][name^="perm-"]:checked').forEach(checkbox => {
            permissions.push(checkbox.value);
        });

        const manifest = {
            id: formData.get('id'),
            name: formData.get('name'),
            version: formData.get('version'),
            description: formData.get('description'),
            author: formData.get('author'),
            main: 'index.js',
            activationEvents: ['onStartup'],
            permissions: permissions,
        };

        const source = formData.get('source');

        try {
            await this.extensionHost.installExtension(manifest, source);
            this.showNotification('Extension created and installed!', 'success');
            this.switchView('installed');
        } catch (error) {
            this.showNotification(`Failed to create extension: ${error.message}`, 'error');
        }
    }

    loadTemplate() {
        const template = `// BerryCode Extension Template
// This extension demonstrates basic functionality

// Register a command
berrycode.commands.registerCommand('helloWorld', () => {
    berrycode.ui.showNotification('Hello from your extension!', 'info');
});

// Add a toolbar button
berrycode.ui.addToolbarButton({
    id: 'hello-button',
    title: 'Say Hello',
    icon: '👋',
    commandId: 'helloWorld'
});

// Listen to file open events
berrycode.editor.onDidOpenFile((file) => {
    console.log('File opened:', file);
});

// Log activation
console.log('Extension activated!');
`;

        document.getElementById('ext-source').value = template;
    }

    searchExtensions(query) {
        // Simple client-side search
        const cards = document.querySelectorAll('.extension-card');
        const lowerQuery = query.toLowerCase();

        cards.forEach(card => {
            const name = card.querySelector('.extension-name').textContent.toLowerCase();
            const desc = card.querySelector('.extension-description').textContent.toLowerCase();

            if (name.includes(lowerQuery) || desc.includes(lowerQuery)) {
                card.style.display = '';
            } else {
                card.style.display = 'none';
            }
        });
    }

    async installFromMarketplace(extensionId) {
        // Sample extension templates
        const templates = {
            'prettier-format': {
                manifest: {
                    id: 'prettier-format',
                    name: 'Prettier Code Formatter',
                    version: '1.0.0',
                    description: 'Format your code with Prettier',
                    author: 'BerryCode Team',
                    main: 'index.js',
                    activationEvents: ['onStartup'],
                    permissions: ['fileSystem:read', 'fileSystem:write'],
                    contributes: {
                        commands: [{
                            id: 'format',
                            title: 'Format Document',
                            keybinding: 'Ctrl+Shift+F'
                        }]
                    }
                },
                source: `berrycode.commands.registerCommand('format', async () => {
    const file = berrycode.editor.getActiveFile();
    if (!file) {
        berrycode.ui.showNotification('No file open', 'warning');
        return;
    }

    const content = await berrycode.editor.getFileContent(file);
    // Simplified formatting (in real impl, use prettier library)
    const formatted = content.replace(/\\s+$/gm, '').replace(/\\n{3,}/g, '\\n\\n');
    await berrycode.editor.setFileContent(file, formatted);

    berrycode.ui.showNotification('File formatted', 'success');
});

console.log('Prettier extension loaded');`
            },
            'git-blame': {
                manifest: {
                    id: 'git-blame',
                    name: 'Git Blame Viewer',
                    version: '1.0.0',
                    description: 'Show git blame information inline',
                    author: 'BerryCode Team',
                    main: 'index.js',
                    activationEvents: ['onStartup'],
                    permissions: ['git:read'],
                    contributes: {
                        commands: [{
                            id: 'blame',
                            title: 'Show Git Blame',
                        }]
                    }
                },
                source: `berrycode.commands.registerCommand('blame', async () => {
    berrycode.ui.showNotification('Git blame feature coming soon!', 'info');
});

berrycode.ui.addToolbarButton({
    id: 'git-blame-btn',
    title: 'Git Blame',
    icon: '📝',
    commandId: 'blame'
});`
            },
            'todo-highlighter': {
                manifest: {
                    id: 'todo-highlighter',
                    name: 'TODO Highlighter',
                    version: '1.2.0',
                    description: 'Highlight TODO, FIXME, and NOTE comments',
                    author: 'Community',
                    main: 'index.js',
                    activationEvents: ['onStartup'],
                    permissions: ['fileSystem:read'],
                },
                source: `berrycode.editor.onDidOpenFile((file) => {
    console.log('Scanning for TODOs in:', file);
    // TODO: Implement highlighting logic
});

console.log('TODO Highlighter loaded');`
            }
        };

        const template = templates[extensionId];
        if (!template) {
            this.showNotification('Extension not found in marketplace', 'error');
            return;
        }

        try {
            await this.extensionHost.installExtension(template.manifest, template.source);
            this.showNotification(`${template.manifest.name} installed!`, 'success');
            this.render();
        } catch (error) {
            this.showNotification(`Installation failed: ${error.message}`, 'error');
        }
    }

    showNotification(message, type = 'info') {
        // Create notification element
        const notification = document.createElement('div');
        notification.className = `notification notification-${type}`;
        notification.textContent = message;

        // Add to body
        document.body.appendChild(notification);

        // Animate in
        setTimeout(() => notification.classList.add('show'), 10);

        // Remove after 3 seconds
        setTimeout(() => {
            notification.classList.remove('show');
            setTimeout(() => notification.remove(), 300);
        }, 3000);
    }

    addToolbarButton(config) {
        console.log('[ExtensionsUI] Adding toolbar button:', config);
        // This would integrate with your existing toolbar
        // For now, just log it
    }

    addSidebarPanel(config) {
        console.log('[ExtensionsUI] Adding sidebar panel:', config);
        // This would integrate with your existing sidebar
        // For now, just log it
    }
}

// Export for use in other scripts
if (typeof window !== 'undefined') {
    window.ExtensionsUI = ExtensionsUI;
}
