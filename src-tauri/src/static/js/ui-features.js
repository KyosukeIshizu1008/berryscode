// UI/UX Enhancement Features for BerryCode
// Zen Mode, Output Channel, Keyboard Shortcuts, Task Runner

// ============================================
// Zen Mode Feature
// ============================================
let isZenMode = false;
let zenModeAwaitingKey = false;

function toggleZenMode() {
    isZenMode = !isZenMode;
    const activityBar = document.querySelector('.activity-bar');
    const sidebar = document.querySelector('.file-sidebar');
    const statusBar = document.querySelector('.status-bar');
    const chatSidebar = document.querySelector('.chat-sidebar');

    if (isZenMode) {
        // Hide all panels
        if (activityBar) activityBar.style.display = 'none';
        if (sidebar) sidebar.style.display = 'none';
        if (statusBar) statusBar.style.display = 'none';
        if (chatSidebar) chatSidebar.style.display = 'none';

        // Make editor fullscreen
        const editorContainer = document.querySelector('.editor-container');
        if (editorContainer) {
            editorContainer.style.width = '100%';
            editorContainer.style.maxWidth = '100%';
        }

        // Add zen mode class to body
        document.body.classList.add('zen-mode');

        // Show notification
        showNotification('Zen Mode enabled. Press ESC to exit.');
    } else {
        // Restore all panels
        if (activityBar) activityBar.style.display = '';
        if (sidebar) sidebar.style.display = '';
        if (statusBar) statusBar.style.display = '';
        if (chatSidebar) chatSidebar.style.display = '';

        // Restore editor container
        const editorContainer = document.querySelector('.editor-container');
        if (editorContainer) {
            editorContainer.style.width = '';
            editorContainer.style.maxWidth = '';
        }

        // Remove zen mode class
        document.body.classList.remove('zen-mode');

        showNotification('Zen Mode disabled.');
    }

    // Trigger Monaco editor layout update
    if (typeof editor !== 'undefined' && editor) {
        setTimeout(() => {
            editor.layout();
        }, 100);
    }
}

// ============================================
// Output Channel Feature
// ============================================
const outputChannels = {
    'build': { name: 'Build', content: [], color: '#007acc' },
    'test': { name: 'Test', content: [], color: '#6c6' },
    'debug': { name: 'Debug', content: [], color: '#f48771' },
    'git': { name: 'Git', content: [], color: '#f34f29' },
    'tasks': { name: 'Tasks', content: [], color: '#c586c0' },
    'general': { name: 'General', content: [], color: '#dcdcaa' }
};

let activeOutputChannel = 'general';
let outputScrollLocked = false;
let outputWordWrap = false;

function createOutputPanel() {
    const existingPanel = document.getElementById('output-panel');
    if (existingPanel) return;

    const outputPanel = document.createElement('div');
    outputPanel.id = 'output-panel';
    outputPanel.className = 'output-panel';
    outputPanel.innerHTML = `
        <div class="output-header">
            <div class="output-header-left">
                <span class="output-title">OUTPUT</span>
                <select class="output-channel-selector" id="output-channel-selector" onchange="switchOutputChannel(this.value)">
                    ${Object.entries(outputChannels).map(([key, channel]) =>
                        `<option value="${key}">${channel.name}</option>`
                    ).join('')}
                </select>
            </div>
            <div class="output-header-right">
                <button class="output-btn" onclick="toggleOutputWordWrap()" title="Toggle Word Wrap">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M12 3v2h-2v1h2c.55 0 1 .45 1 1v2c0 .55-.45 1-1 1H9v-1h3V7H9c-.55 0-1-.45-1-1V4c0-.55.45-1 1-1h3zM6 3v2H3v1h3c.55 0 1 .45 1 1v2c0 .55-.45 1-1 1H3v-1h3V7H3c-.55 0-1-.45-1-1V4c0-.55.45-1 1-1h3z"/>
                    </svg>
                </button>
                <button class="output-btn" onclick="toggleOutputScrollLock()" title="Scroll Lock">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M8 2a1 1 0 0 1 1 1v5h5a1 1 0 1 1 0 2H9v5a1 1 0 1 1-2 0V10H2a1 1 0 1 1 0-2h5V3a1 1 0 0 1 1-1z"/>
                    </svg>
                </button>
                <button class="output-btn" onclick="clearOutputChannel()" title="Clear Output">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M8 1a2 2 0 0 1 2 2v2h3.5a.5.5 0 0 1 0 1h-.441l-.443 6.189A2 2 0 0 1 10.623 14H5.377a2 2 0 0 1-1.993-1.811L2.941 6H2.5a.5.5 0 0 1 0-1H6V3a2 2 0 0 1 2-2zm0 1a1 1 0 0 0-1 1v2h2V3a1 1 0 0 0-1-1zm-3 5l.411 5.743A1 1 0 0 0 5.377 13h5.246a1 1 0 0 0 .966-.743L12 7H5z"/>
                    </svg>
                </button>
                <button class="output-btn" onclick="toggleOutputPanel()" title="Close">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M4.646 4.646a.5.5 0 0 1 .708 0L8 7.293l2.646-2.647a.5.5 0 0 1 .708.708L8.707 8l2.647 2.646a.5.5 0 0 1-.708.708L8 8.707l-2.646 2.647a.5.5 0 0 1-.708-.708L7.293 8 4.646 5.354a.5.5 0 0 1 0-.708z"/>
                    </svg>
                </button>
            </div>
        </div>
        <div class="output-content" id="output-content">
            <pre id="output-text"></pre>
        </div>
    `;

    document.body.appendChild(outputPanel);
}

function toggleOutputPanel() {
    const panel = document.getElementById('output-panel');
    if (panel) {
        panel.style.display = panel.style.display === 'none' ? 'flex' : 'none';
    } else {
        createOutputPanel();
    }
}

function switchOutputChannel(channelId) {
    activeOutputChannel = channelId;
    updateOutputDisplay();
}

function appendToOutput(channelId, message, type = 'info') {
    if (!outputChannels[channelId]) {
        channelId = 'general';
    }

    const timestamp = new Date().toLocaleTimeString();
    const line = {
        timestamp,
        message,
        type
    };

    outputChannels[channelId].content.push(line);

    // Keep only last 1000 lines per channel
    if (outputChannels[channelId].content.length > 1000) {
        outputChannels[channelId].content.shift();
    }

    if (activeOutputChannel === channelId) {
        updateOutputDisplay();
    }
}

function updateOutputDisplay() {
    const outputText = document.getElementById('output-text');
    if (!outputText) return;

    const channel = outputChannels[activeOutputChannel];
    if (!channel) return;

    // Convert ANSI codes to HTML
    const formattedContent = channel.content.map(line => {
        const className = `output-${line.type}`;
        return `<span class="${className}">[${line.timestamp}] ${escapeHtml(line.message)}</span>`;
    }).join('\n');

    outputText.innerHTML = formattedContent;

    // Auto-scroll to bottom if not locked
    if (!outputScrollLocked) {
        const outputContent = document.getElementById('output-content');
        if (outputContent) {
            outputContent.scrollTop = outputContent.scrollHeight;
        }
    }
}

function clearOutputChannel() {
    if (outputChannels[activeOutputChannel]) {
        outputChannels[activeOutputChannel].content = [];
        updateOutputDisplay();
    }
}

function toggleOutputScrollLock() {
    outputScrollLocked = !outputScrollLocked;
    const btn = event.currentTarget;
    btn.style.opacity = outputScrollLocked ? '1' : '0.6';
}

function toggleOutputWordWrap() {
    outputWordWrap = !outputWordWrap;
    const outputText = document.getElementById('output-text');
    if (outputText) {
        outputText.style.whiteSpace = outputWordWrap ? 'pre-wrap' : 'pre';
    }
}

// ============================================
// Enhanced Keyboard Shortcuts with Conflict Detection
// ============================================
function detectKeybindingConflicts() {
    const conflicts = [];
    const keybindings = Object.values(customKeybindings).concat(Object.values(defaultKeybindings));
    const keyMap = new Map();

    keybindings.forEach((binding, index) => {
        const key = `${binding.mac || binding.key}`;
        if (keyMap.has(key)) {
            conflicts.push({
                key,
                commands: [keyMap.get(key), binding.command]
            });
        } else {
            keyMap.set(key, binding.command);
        }
    });

    return conflicts;
}

function renderShortcuts() {
    const shortcutsList = document.getElementById('shortcuts-list');
    if (!shortcutsList) return;

    const conflicts = detectKeybindingConflicts();
    const conflictKeys = new Set(conflicts.map(c => c.key));

    // Merge default and custom keybindings
    const allKeybindings = { ...defaultKeybindings, ...customKeybindings };

    shortcutsList.innerHTML = Object.entries(allKeybindings).map(([id, binding]) => {
        const key = binding.mac || binding.key;
        const isConflict = conflictKeys.has(key);
        const conflictClass = isConflict ? 'shortcut-conflict' : '';

        return `
            <div class="shortcut-item ${conflictClass}" data-id="${id}">
                <div class="shortcut-info">
                    <div class="shortcut-command">${binding.command}</div>
                    <div class="shortcut-description">${binding.description}</div>
                    ${isConflict ? '<span class="conflict-badge">Conflict</span>' : ''}
                </div>
                <div class="shortcut-key-wrapper">
                    <kbd class="shortcut-key" onclick="editKeybinding('${id}')">${formatKeyBinding(key)}</kbd>
                    ${customKeybindings[id] ? `<button class="shortcut-reset-btn" onclick="resetSingleKeybinding('${id}')">Reset</button>` : ''}
                </div>
            </div>
        `;
    }).join('');
}

function editKeybinding(id) {
    const binding = customKeybindings[id] || defaultKeybindings[id];
    if (!binding) return;

    const newKey = prompt(`Enter new keyboard shortcut for "${binding.command}"\nFormat: Cmd+K or Ctrl+Shift+P`, binding.mac || binding.key);
    if (newKey && newKey.trim()) {
        customKeybindings[id] = {
            ...binding,
            key: newKey.trim(),
            mac: newKey.trim()
        };
        saveKeybindings();
        renderShortcuts();
    }
}

function resetSingleKeybinding(id) {
    delete customKeybindings[id];
    saveKeybindings();
    renderShortcuts();
}

function filterShortcuts(query) {
    const items = document.querySelectorAll('.shortcut-item');
    const lowerQuery = query.toLowerCase();

    items.forEach(item => {
        const command = item.querySelector('.shortcut-command').textContent.toLowerCase();
        const description = item.querySelector('.shortcut-description').textContent.toLowerCase();
        const key = item.querySelector('.shortcut-key').textContent.toLowerCase();

        if (command.includes(lowerQuery) || description.includes(lowerQuery) || key.includes(lowerQuery)) {
            item.style.display = '';
        } else {
            item.style.display = 'none';
        }
    });
}

function formatKeyBinding(key) {
    return key
        .replace(/Cmd/g, '⌘')
        .replace(/Ctrl/g, 'Ctrl')
        .replace(/Shift/g, '⇧')
        .replace(/Alt/g, '⌥')
        .replace(/\+/g, ' + ');
}

// ============================================
// Task Runner Integration
// ============================================
let availableTasks = [];
let runningTasks = new Map();

async function loadTasks() {
    try {
        const response = await fetch(`/api/tasks/list?session_id=${sessionId}&project_path=${encodeURIComponent(projectRoot)}`);
        const data = await response.json();

        if (data.success) {
            availableTasks = data.tasks;
            renderTasksList();
        }
    } catch (error) {
        console.error('Failed to load tasks:', error);
    }
}

function renderTasksList() {
    const tasksList = document.getElementById('tasks-list');
    if (!tasksList) return;

    tasksList.innerHTML = availableTasks.map(task => `
        <div class="task-item">
            <div class="task-info">
                <div class="task-label">${task.label}</div>
                <div class="task-command">${task.command}</div>
            </div>
            <button class="task-run-btn" onclick="runTask('${task.label}')">
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                    <path d="M3 2v12l10-6L3 2z"/>
                </svg>
                Run
            </button>
        </div>
    `).join('');
}

async function runTask(taskLabel) {
    const task = availableTasks.find(t => t.label === taskLabel);
    if (!task) return;

    const channelId = task.group || 'tasks';
    appendToOutput(channelId, `Starting task: ${taskLabel}`, 'info');

    // Show output panel
    const panel = document.getElementById('output-panel');
    if (panel) {
        panel.style.display = 'flex';
        switchOutputChannel(channelId);
    } else {
        createOutputPanel();
        switchOutputChannel(channelId);
    }

    try {
        const response = await fetch('/api/tasks/run', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                session_id: sessionId,
                project_path: projectRoot,
                task: task
            })
        });

        const data = await response.json();

        if (data.success) {
            appendToOutput(channelId, `Task started: ${taskLabel} (ID: ${data.task_id})`, 'info');
            runningTasks.set(data.task_id, { task, channelId });

            // Poll for task output
            pollTaskOutput(data.task_id, channelId);
        } else {
            appendToOutput(channelId, `Failed to start task: ${data.message}`, 'error');
        }
    } catch (error) {
        appendToOutput(channelId, `Error running task: ${error.message}`, 'error');
    }
}

async function pollTaskOutput(taskId, channelId) {
    const pollInterval = setInterval(async () => {
        try {
            const response = await fetch(`/api/tasks/output?task_id=${taskId}`);
            const data = await response.json();

            if (data.success && data.execution) {
                const execution = data.execution;

                if (execution.status === 'completed' || execution.status === 'failed') {
                    clearInterval(pollInterval);

                    appendToOutput(channelId, execution.output, 'info');

                    if (execution.status === 'completed') {
                        appendToOutput(channelId, `Task completed successfully (exit code: ${execution.exit_code})`, 'success');
                    } else {
                        appendToOutput(channelId, `Task failed (exit code: ${execution.exit_code})`, 'error');
                    }

                    runningTasks.delete(taskId);
                }
            }
        } catch (error) {
            clearInterval(pollInterval);
            appendToOutput(channelId, `Error polling task output: ${error.message}`, 'error');
        }
    }, 1000);
}

// Helper function
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function showNotification(message) {
    // Use existing notification system if available
    if (typeof window.showNotification === 'function') {
        window.showNotification(message);
    } else {
        console.log(message);
    }
}

// Initialize on page load
if (typeof document !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        // Add keyboard shortcut handlers
        document.addEventListener('keydown', (e) => {
            // Cmd+K Z - Toggle Zen Mode
            if ((e.metaKey || e.ctrlKey) && e.key === 'k' && !zenModeAwaitingKey) {
                zenModeAwaitingKey = true;
                setTimeout(() => {
                    zenModeAwaitingKey = false;
                }, 1000);
                e.preventDefault();
            }
            else if (zenModeAwaitingKey && e.key === 'z') {
                toggleZenMode();
                zenModeAwaitingKey = false;
                e.preventDefault();
            }
            // ESC - Exit Zen Mode
            else if (e.key === 'Escape' && isZenMode) {
                toggleZenMode();
                e.preventDefault();
            }
        });

        // Load tasks if project is loaded
        if (typeof sessionId !== 'undefined' && typeof projectRoot !== 'undefined') {
            loadTasks();
        }
    });
}
