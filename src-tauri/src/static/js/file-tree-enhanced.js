/**
 * Enhanced File Tree Component for BerryCode
 * Phase 4 Implementation - Improved Sidebar/File Tree
 */

class EnhancedFileTree {
    constructor(containerSelector, sessionId) {
        this.container = document.querySelector(containerSelector);
        this.sessionId = sessionId;
        this.expandedFolders = new Set();
        this.fileTreeData = null;
        this.selectedFile = null;

        this.init();
    }

    async init() {
        await this.loadFileTree();
        this.setupEventListeners();
    }

    /**
     * Load file tree from API
     */
    async loadFileTree() {
        try {
            const response = await fetch(`/api/files/tree?session_id=${this.sessionId}`);
            if (response.ok) {
                this.fileTreeData = await response.json();
                // Expand root folder by default
                if (this.fileTreeData && this.fileTreeData.path) {
                    this.expandedFolders.add(this.fileTreeData.path);
                }
                this.render();
            } else {
                this.showError('Failed to load file tree');
            }
        } catch (error) {
            console.error('Error loading file tree:', error);
            this.showError('Error loading file tree');
        }
    }

    /**
     * Render the entire file tree
     */
    render() {
        if (!this.container) return;

        this.container.innerHTML = '';

        if (!this.fileTreeData) {
            this.showError('No file tree data');
            return;
        }

        this.renderNode(this.fileTreeData, 0);
    }

    /**
     * Render a single node (file or folder)
     */
    renderNode(node, level) {
        if (!node.name && level === 0) {
            // Root node, render children
            if (node.children && node.children.length > 0) {
                this.renderChildren(node, level);
            }
            return;
        }

        const nodeElement = document.createElement('div');
        nodeElement.className = 'file-tree-node';
        nodeElement.dataset.path = node.path;
        nodeElement.dataset.isDir = node.is_dir;

        // Calculate indentation
        const indent = level * 16;
        nodeElement.style.paddingLeft = `${indent + 8}px`;

        if (node.is_dir) {
            this.renderFolderNode(nodeElement, node, level);
        } else {
            this.renderFileNode(nodeElement, node);
        }

        this.container.appendChild(nodeElement);

        // Render children if folder is expanded
        if (node.is_dir && this.expandedFolders.has(node.path)) {
            this.renderChildren(node, level);
        }
    }

    /**
     * Render folder node
     */
    renderFolderNode(element, node, level) {
        const isExpanded = this.expandedFolders.has(node.path);

        element.classList.add('folder-node');
        if (isExpanded) element.classList.add('expanded');

        element.innerHTML = `
            <div class="node-content">
                <i class="codicon ${isExpanded ? 'codicon-chevron-down' : 'codicon-chevron-right'} expand-icon"></i>
                <i class="codicon ${isExpanded ? 'codicon-folder-opened' : 'codicon-folder'} folder-icon"></i>
                <span class="node-label">${this.escapeHtml(node.name)}</span>
                <span class="node-count">${node.children ? node.children.length : 0}</span>
            </div>
        `;

        // Folder click handler
        element.querySelector('.node-content').addEventListener('click', () => {
            this.toggleFolder(node);
        });

        // Context menu
        element.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            this.showContextMenu(e, node, true);
        });
    }

    /**
     * Render file node
     */
    renderFileNode(element, node) {
        element.classList.add('file-node');

        if (this.selectedFile === node.path) {
            element.classList.add('selected');
        }

        // Git status styling
        let statusClass = '';
        let statusIndicator = '';

        if (node.git_status) {
            statusClass = `git-${node.git_status}`;
            const statusIcons = {
                'modified': 'M',
                'staged': 'A',
                'untracked': 'U',
                'conflict': '!'
            };
            statusIndicator = `<span class="git-status-indicator ${statusClass}">${statusIcons[node.git_status] || ''}</span>`;
        }

        const fileIcon = this.getFileIcon(node.name);

        element.innerHTML = `
            <div class="node-content ${statusClass}">
                <span class="indent-spacer"></span>
                ${statusIndicator}
                <i class="codicon ${fileIcon} file-icon"></i>
                <span class="node-label">${this.escapeHtml(node.name)}</span>
            </div>
        `;

        // File click handler - single click to open
        element.querySelector('.node-content').addEventListener('click', () => {
            this.selectFile(node);
            this.openFile(node); // Open immediately on single click
        });

        // Context menu
        element.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            this.showContextMenu(e, node, false);
        });

        // Drag support
        element.draggable = true;
        element.addEventListener('dragstart', (e) => {
            this.handleDragStart(e, node);
        });
    }

    /**
     * Render children nodes
     */
    renderChildren(node, level) {
        if (!node.children || node.children.length === 0) return;

        // Sort: directories first, then files
        const sorted = [...node.children].sort((a, b) => {
            if (a.is_dir === b.is_dir) {
                return a.name.localeCompare(b.name);
            }
            return a.is_dir ? -1 : 1;
        });

        sorted.forEach(child => {
            this.renderNode(child, level + 1);
        });
    }

    /**
     * Toggle folder expansion
     */
    toggleFolder(node) {
        if (this.expandedFolders.has(node.path)) {
            this.expandedFolders.delete(node.path);
        } else {
            this.expandedFolders.add(node.path);
        }
        this.render();
    }

    /**
     * Select a file
     */
    selectFile(node) {
        // Remove previous selection
        const previousSelected = this.container.querySelector('.file-tree-node.selected');
        if (previousSelected) {
            previousSelected.classList.remove('selected');
        }

        // Add selection to current node
        this.selectedFile = node.path;
        const currentNode = this.container.querySelector(`[data-path="${node.path}"]`);
        if (currentNode) {
            currentNode.classList.add('selected');
        }
    }

    /**
     * Open a file
     */
    async openFile(node) {
        if (typeof loadFile === 'function') {
            await loadFile(node.path);
        } else {
            console.warn('loadFile function not found');
        }
    }

    /**
     * Show context menu
     */
    showContextMenu(event, node, isFolder) {
        event.preventDefault();

        // Remove existing context menu
        const existing = document.querySelector('.file-tree-context-menu');
        if (existing) existing.remove();

        const menu = document.createElement('div');
        menu.className = 'file-tree-context-menu';
        menu.style.left = `${event.pageX}px`;
        menu.style.top = `${event.pageY}px`;

        const menuItems = [];

        if (isFolder) {
            menuItems.push(
                { label: 'New File', icon: 'codicon-new-file', action: () => this.createNewFile(node.path) },
                { label: 'New Folder', icon: 'codicon-new-folder', action: () => this.createNewFolder(node.path) },
                { separator: true },
                { label: 'Rename', icon: 'codicon-edit', action: () => this.renameItem(node) },
                { label: 'Delete', icon: 'codicon-trash', action: () => this.deleteItem(node), danger: true }
            );
        } else {
            menuItems.push(
                { label: 'Open', icon: 'codicon-go-to-file', action: () => this.openFile(node) },
                { label: 'Open to Side', icon: 'codicon-split-horizontal', action: () => this.openFileToSide(node) },
                { separator: true },
                { label: 'Copy Path', icon: 'codicon-copy', action: () => this.copyPath(node.path) },
                { label: 'Copy Relative Path', icon: 'codicon-copy', action: () => this.copyRelativePath(node.path) },
                { separator: true },
                { label: 'Rename', icon: 'codicon-edit', action: () => this.renameItem(node) },
                { label: 'Delete', icon: 'codicon-trash', action: () => this.deleteItem(node), danger: true }
            );
        }

        menuItems.forEach(item => {
            if (item.separator) {
                const separator = document.createElement('div');
                separator.className = 'context-menu-separator';
                menu.appendChild(separator);
            } else {
                const menuItem = document.createElement('div');
                menuItem.className = 'context-menu-item';
                if (item.danger) menuItem.classList.add('danger');

                menuItem.innerHTML = `
                    <i class="codicon ${item.icon}"></i>
                    <span>${item.label}</span>
                `;

                menuItem.addEventListener('click', () => {
                    item.action();
                    menu.remove();
                });

                menu.appendChild(menuItem);
            }
        });

        document.body.appendChild(menu);

        // Close menu on click outside
        const closeMenu = (e) => {
            if (!menu.contains(e.target)) {
                menu.remove();
                document.removeEventListener('click', closeMenu);
            }
        };
        setTimeout(() => document.addEventListener('click', closeMenu), 0);
    }

    /**
     * Create new file
     */
    async createNewFile(folderPath) {
        const fileName = prompt('Enter file name:');
        if (!fileName) return;

        try {
            const fullPath = `${folderPath}/${fileName}`;
            const response = await fetch(`/api/files?session_id=${this.sessionId}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ path: fullPath, content: '' })
            });

            if (response.ok) {
                await this.loadFileTree();
                this.expandedFolders.add(folderPath);
                this.render();
            } else {
                alert('Failed to create file');
            }
        } catch (error) {
            console.error('Error creating file:', error);
            alert('Error creating file');
        }
    }

    /**
     * Create new folder
     */
    async createNewFolder(parentPath) {
        const folderName = prompt('Enter folder name:');
        if (!folderName) return;

        try {
            const fullPath = `${parentPath}/${folderName}`;
            const response = await fetch(`/api/files/mkdir?session_id=${this.sessionId}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ path: fullPath })
            });

            if (response.ok) {
                await this.loadFileTree();
                this.expandedFolders.add(parentPath);
                this.render();
            } else {
                alert('Failed to create folder');
            }
        } catch (error) {
            console.error('Error creating folder:', error);
            alert('Error creating folder');
        }
    }

    /**
     * Rename item
     */
    async renameItem(node) {
        const newName = prompt('Enter new name:', node.name);
        if (!newName || newName === node.name) return;

        try {
            const parentPath = node.path.substring(0, node.path.lastIndexOf('/'));
            const newPath = `${parentPath}/${newName}`;

            const response = await fetch(`/api/files/rename?session_id=${this.sessionId}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ old_path: node.path, new_path: newPath })
            });

            if (response.ok) {
                await this.loadFileTree();
                this.render();
            } else {
                alert('Failed to rename item');
            }
        } catch (error) {
            console.error('Error renaming item:', error);
            alert('Error renaming item');
        }
    }

    /**
     * Delete item
     */
    async deleteItem(node) {
        const confirmMsg = node.is_dir
            ? `Are you sure you want to delete folder "${node.name}" and all its contents?`
            : `Are you sure you want to delete file "${node.name}"?`;

        if (!confirm(confirmMsg)) return;

        try {
            const response = await fetch(`/api/files?session_id=${this.sessionId}&path=${encodeURIComponent(node.path)}`, {
                method: 'DELETE'
            });

            if (response.ok) {
                await this.loadFileTree();
                this.render();
            } else {
                alert('Failed to delete item');
            }
        } catch (error) {
            console.error('Error deleting item:', error);
            alert('Error deleting item');
        }
    }

    /**
     * Copy path to clipboard
     */
    async copyPath(path) {
        try {
            await navigator.clipboard.writeText(path);
            console.log('Path copied:', path);
        } catch (error) {
            console.error('Failed to copy path:', error);
        }
    }

    /**
     * Copy relative path to clipboard
     */
    async copyRelativePath(path) {
        // Extract relative path from full path
        const parts = path.split('/');
        const relativePath = parts.slice(parts.length - 3).join('/');

        try {
            await navigator.clipboard.writeText(relativePath);
            console.log('Relative path copied:', relativePath);
        } catch (error) {
            console.error('Failed to copy relative path:', error);
        }
    }

    /**
     * Open file to side (split view)
     */
    openFileToSide(node) {
        // TODO: Implement split view
        console.log('Open to side:', node.path);
        this.openFile(node);
    }

    /**
     * Handle drag start
     */
    handleDragStart(event, node) {
        event.dataTransfer.effectAllowed = 'move';
        event.dataTransfer.setData('text/plain', node.path);
        event.dataTransfer.setData('application/json', JSON.stringify(node));
    }

    /**
     * Get appropriate icon for file type
     */
    getFileIcon(fileName) {
        const ext = fileName.split('.').pop().toLowerCase();

        const iconMap = {
            // Programming languages
            'js': 'codicon-file-code',
            'ts': 'codicon-file-code',
            'jsx': 'codicon-file-code',
            'tsx': 'codicon-file-code',
            'rs': 'codicon-file-code',
            'py': 'codicon-file-code',
            'java': 'codicon-file-code',
            'cpp': 'codicon-file-code',
            'c': 'codicon-file-code',
            'go': 'codicon-file-code',
            'rb': 'codicon-file-code',

            // Web
            'html': 'codicon-file-code',
            'css': 'codicon-file-code',
            'scss': 'codicon-file-code',
            'sass': 'codicon-file-code',
            'vue': 'codicon-file-code',

            // Data
            'json': 'codicon-file-code',
            'xml': 'codicon-file-code',
            'yaml': 'codicon-file-code',
            'yml': 'codicon-file-code',
            'toml': 'codicon-file-code',

            // Documents
            'md': 'codicon-markdown',
            'txt': 'codicon-file-text',
            'pdf': 'codicon-file-pdf',

            // Images
            'png': 'codicon-file-media',
            'jpg': 'codicon-file-media',
            'jpeg': 'codicon-file-media',
            'gif': 'codicon-file-media',
            'svg': 'codicon-file-media',

            // Archives
            'zip': 'codicon-file-zip',
            'tar': 'codicon-file-zip',
            'gz': 'codicon-file-zip',

            // Binary
            'exe': 'codicon-file-binary',
            'dll': 'codicon-file-binary',
            'so': 'codicon-file-binary'
        };

        return iconMap[ext] || 'codicon-file';
    }

    /**
     * Show error message
     */
    showError(message) {
        if (!this.container) return;

        this.container.innerHTML = `
            <div style="padding: 16px; color: var(--text-tertiary); text-align: center;">
                <i class="codicon codicon-error" style="font-size: 32px; display: block; margin-bottom: 8px;"></i>
                <div>${this.escapeHtml(message)}</div>
            </div>
        `;
    }

    /**
     * Escape HTML to prevent XSS
     */
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    /**
     * Setup global event listeners
     */
    setupEventListeners() {
        // Refresh on file system changes
        document.addEventListener('fileTreeChanged', () => {
            this.loadFileTree();
        });
    }
}

// Export for global use
window.EnhancedFileTree = EnhancedFileTree;
