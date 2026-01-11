/**
 * Git Graph Visualization
 * Interactive commit graph using SVG
 */

class GitGraph {
    constructor(containerId) {
        this.container = document.getElementById(containerId);
        this.commits = [];
        this.branches = new Map();
        this.svg = null;
        this.width = 0;
        this.height = 0;
        this.commitRadius = 6;
        this.rowHeight = 50;
        this.columnWidth = 30;
        this.colors = [
            '#0ea5e9', // sky-500
            '#22c55e', // green-500
            '#f59e0b', // amber-500
            '#ef4444', // red-500
            '#8b5cf6', // violet-500
            '#ec4899', // pink-500
            '#14b8a6', // teal-500
            '#f97316', // orange-500
        ];
        this.colorIndex = 0;
    }

    async loadGraph(sessionId) {
        try {
            const response = await fetch(`/api/git/commit-graph?session_id=${sessionId}`);
            if (!response.ok) throw new Error('Failed to fetch commit graph');

            this.commits = await response.json();
            this.render();
        } catch (error) {
            console.error('Error loading git graph:', error);
            this.showError('Failed to load git graph');
        }
    }

    render() {
        if (!this.container) return;

        // Clear container
        this.container.innerHTML = '';

        if (this.commits.length === 0) {
            this.container.innerHTML = '<div style="padding: 20px; text-align: center; color: var(--text-tertiary);">No commits to display</div>';
            return;
        }

        // Calculate dimensions
        this.width = this.container.clientWidth;
        this.height = Math.max(600, this.commits.length * this.rowHeight + 100);

        // Create SVG
        this.svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        this.svg.setAttribute('width', this.width);
        this.svg.setAttribute('height', this.height);
        this.svg.style.background = 'var(--bg-tertiary)';

        // Build graph structure
        this.buildGraphStructure();

        // Draw connections
        this.drawConnections();

        // Draw commits
        this.drawCommits();

        this.container.appendChild(this.svg);
    }

    buildGraphStructure() {
        // Assign columns to commits based on parent relationships
        const commitMap = new Map();
        const columnAssignments = new Map();
        let nextColumn = 0;

        this.commits.forEach((commit, index) => {
            commitMap.set(commit.sha, { commit, index });
        });

        this.commits.forEach((commit, index) => {
            let column;

            if (commit.parent_shas.length === 0) {
                // Root commit
                column = nextColumn++;
            } else if (commit.parent_shas.length === 1) {
                // Single parent - try to use parent's column
                const parentSha = commit.parent_shas[0];
                const parentData = commitMap.get(parentSha);

                if (parentData && columnAssignments.has(parentSha)) {
                    column = columnAssignments.get(parentSha);
                } else {
                    column = nextColumn++;
                }
            } else {
                // Merge commit - use first parent's column
                const parentSha = commit.parent_shas[0];
                if (columnAssignments.has(parentSha)) {
                    column = columnAssignments.get(parentSha);
                } else {
                    column = nextColumn++;
                }
            }

            columnAssignments.set(commit.sha, column);
            commit.column = column;
            commit.row = index;

            // Assign colors to branches
            if (commit.branches && commit.branches.length > 0) {
                commit.branches.forEach(branch => {
                    if (!this.branches.has(branch)) {
                        this.branches.set(branch, this.getNextColor());
                    }
                });
            }
        });
    }

    drawConnections() {
        const commitMap = new Map();
        this.commits.forEach(commit => {
            commitMap.set(commit.sha, commit);
        });

        this.commits.forEach(commit => {
            if (commit.parent_shas.length === 0) return;

            const x1 = this.columnWidth * commit.column + 100;
            const y1 = this.rowHeight * commit.row + 50;

            commit.parent_shas.forEach((parentSha, index) => {
                const parent = commitMap.get(parentSha);
                if (!parent) return;

                const x2 = this.columnWidth * parent.column + 100;
                const y2 = this.rowHeight * parent.row + 50;

                // Create path
                const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');

                // Bezier curve for smooth connections
                const midY = (y1 + y2) / 2;
                const d = `M ${x1} ${y1} C ${x1} ${midY}, ${x2} ${midY}, ${x2} ${y2}`;

                path.setAttribute('d', d);
                path.setAttribute('stroke', this.getCommitColor(commit));
                path.setAttribute('stroke-width', '2');
                path.setAttribute('fill', 'none');
                path.setAttribute('opacity', '0.6');

                this.svg.appendChild(path);
            });
        });
    }

    drawCommits() {
        this.commits.forEach(commit => {
            const x = this.columnWidth * commit.column + 100;
            const y = this.rowHeight * commit.row + 50;

            // Commit circle
            const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
            circle.setAttribute('cx', x);
            circle.setAttribute('cy', y);
            circle.setAttribute('r', this.commitRadius);
            circle.setAttribute('fill', this.getCommitColor(commit));
            circle.setAttribute('stroke', '#ffffff');
            circle.setAttribute('stroke-width', '2');
            circle.style.cursor = 'pointer';

            // Hover effect
            circle.addEventListener('mouseenter', () => {
                circle.setAttribute('r', this.commitRadius + 2);
                this.showTooltip(commit, x, y);
            });
            circle.addEventListener('mouseleave', () => {
                circle.setAttribute('r', this.commitRadius);
                this.hideTooltip();
            });

            // Click to show details
            circle.addEventListener('click', () => {
                this.showCommitDetails(commit);
            });

            this.svg.appendChild(circle);

            // Commit info text
            const group = document.createElementNS('http://www.w3.org/2000/svg', 'g');

            // SHA
            const shaText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
            shaText.setAttribute('x', x + 15);
            shaText.setAttribute('y', y - 10);
            shaText.setAttribute('fill', 'var(--text-secondary)');
            shaText.setAttribute('font-size', '11');
            shaText.setAttribute('font-family', 'monospace');
            shaText.textContent = commit.short_sha;
            group.appendChild(shaText);

            // Message
            const messageText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
            messageText.setAttribute('x', x + 15);
            messageText.setAttribute('y', y + 5);
            messageText.setAttribute('fill', 'var(--text-primary)');
            messageText.setAttribute('font-size', '12');
            messageText.setAttribute('font-weight', '500');
            const message = commit.message.split('\n')[0];
            messageText.textContent = message.length > 50 ? message.substring(0, 47) + '...' : message;
            group.appendChild(messageText);

            // Author and date
            const authorText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
            authorText.setAttribute('x', x + 15);
            authorText.setAttribute('y', y + 18);
            authorText.setAttribute('fill', 'var(--text-tertiary)');
            authorText.setAttribute('font-size', '10');
            const date = new Date(commit.date * 1000);
            authorText.textContent = `${commit.author} - ${this.formatDate(date)}`;
            group.appendChild(authorText);

            // Branch labels
            if (commit.branches && commit.branches.length > 0) {
                commit.branches.forEach((branch, index) => {
                    const labelRect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
                    const labelText = document.createElementNS('http://www.w3.org/2000/svg', 'text');

                    const labelX = x + 15 + (index * 70);
                    const labelY = y - 25;

                    labelRect.setAttribute('x', labelX);
                    labelRect.setAttribute('y', labelY - 12);
                    labelRect.setAttribute('width', branch.length * 6 + 10);
                    labelRect.setAttribute('height', '16');
                    labelRect.setAttribute('rx', '3');
                    labelRect.setAttribute('fill', this.branches.get(branch) || '#666');
                    labelRect.setAttribute('opacity', '0.9');

                    labelText.setAttribute('x', labelX + 5);
                    labelText.setAttribute('y', labelY);
                    labelText.setAttribute('fill', '#ffffff');
                    labelText.setAttribute('font-size', '10');
                    labelText.setAttribute('font-weight', '600');
                    labelText.textContent = branch;

                    group.appendChild(labelRect);
                    group.appendChild(labelText);
                });
            }

            // Tag labels
            if (commit.tags && commit.tags.length > 0) {
                commit.tags.forEach((tag, index) => {
                    const tagIcon = document.createElementNS('http://www.w3.org/2000/svg', 'text');
                    tagIcon.setAttribute('x', x + 15 + (commit.branches ? commit.branches.length * 70 : 0) + (index * 60));
                    tagIcon.setAttribute('y', y - 25);
                    tagIcon.setAttribute('fill', '#f59e0b');
                    tagIcon.setAttribute('font-size', '12');
                    tagIcon.textContent = `🏷️ ${tag}`;
                    group.appendChild(tagIcon);
                });
            }

            this.svg.appendChild(group);
        });
    }

    getCommitColor(commit) {
        if (commit.branches && commit.branches.length > 0) {
            return this.branches.get(commit.branches[0]) || this.colors[0];
        }
        return this.colors[commit.column % this.colors.length];
    }

    getNextColor() {
        const color = this.colors[this.colorIndex % this.colors.length];
        this.colorIndex++;
        return color;
    }

    formatDate(date) {
        const now = new Date();
        const diff = now - date;
        const days = Math.floor(diff / (1000 * 60 * 60 * 24));

        if (days === 0) {
            const hours = Math.floor(diff / (1000 * 60 * 60));
            if (hours === 0) {
                const minutes = Math.floor(diff / (1000 * 60));
                return `${minutes} minutes ago`;
            }
            return `${hours} hours ago`;
        } else if (days === 1) {
            return 'yesterday';
        } else if (days < 7) {
            return `${days} days ago`;
        } else {
            return date.toLocaleDateString();
        }
    }

    showTooltip(commit, x, y) {
        let tooltip = document.getElementById('git-graph-tooltip');
        if (!tooltip) {
            tooltip = document.createElement('div');
            tooltip.id = 'git-graph-tooltip';
            tooltip.style.position = 'absolute';
            tooltip.style.background = 'var(--bg-primary)';
            tooltip.style.border = '1px solid var(--border-color)';
            tooltip.style.borderRadius = '6px';
            tooltip.style.padding = '12px';
            tooltip.style.boxShadow = '0 4px 12px rgba(0,0,0,0.3)';
            tooltip.style.zIndex = '1000';
            tooltip.style.maxWidth = '300px';
            tooltip.style.fontSize = '12px';
            document.body.appendChild(tooltip);
        }

        tooltip.innerHTML = `
            <div style="font-family: monospace; color: var(--text-secondary); margin-bottom: 4px;">${commit.sha}</div>
            <div style="font-weight: 600; color: var(--text-primary); margin-bottom: 8px;">${commit.message.split('\n')[0]}</div>
            <div style="color: var(--text-tertiary);">
                <div>Author: ${commit.author}</div>
                <div>Email: ${commit.email}</div>
                <div>Date: ${new Date(commit.date * 1000).toLocaleString()}</div>
                ${commit.parent_shas.length > 0 ? `<div>Parents: ${commit.parent_shas.length}</div>` : ''}
            </div>
        `;

        const rect = this.container.getBoundingClientRect();
        tooltip.style.left = (rect.left + x + 20) + 'px';
        tooltip.style.top = (rect.top + y - 50) + 'px';
        tooltip.style.display = 'block';
    }

    hideTooltip() {
        const tooltip = document.getElementById('git-graph-tooltip');
        if (tooltip) {
            tooltip.style.display = 'none';
        }
    }

    showCommitDetails(commit) {
        // Create modal or panel to show commit details
        const modal = document.createElement('div');
        modal.style.position = 'fixed';
        modal.style.top = '0';
        modal.style.left = '0';
        modal.style.width = '100%';
        modal.style.height = '100%';
        modal.style.background = 'rgba(0,0,0,0.7)';
        modal.style.display = 'flex';
        modal.style.alignItems = 'center';
        modal.style.justifyContent = 'center';
        modal.style.zIndex = '10000';

        const content = document.createElement('div');
        content.style.background = 'var(--bg-secondary)';
        content.style.borderRadius = '8px';
        content.style.padding = '24px';
        content.style.maxWidth = '600px';
        content.style.maxHeight = '80vh';
        content.style.overflow = 'auto';
        content.style.boxShadow = '0 8px 32px rgba(0,0,0,0.4)';

        content.innerHTML = `
            <div style="display: flex; justify-content: space-between; align-items: start; margin-bottom: 20px;">
                <h3 style="margin: 0; color: var(--text-primary);">Commit Details</h3>
                <button onclick="this.closest('.commit-modal').remove()" style="background: none; border: none; color: var(--text-tertiary); font-size: 20px; cursor: pointer;">&times;</button>
            </div>
            <div style="color: var(--text-primary);">
                <div style="margin-bottom: 16px;">
                    <div style="color: var(--text-secondary); font-size: 12px; margin-bottom: 4px;">SHA</div>
                    <div style="font-family: monospace; background: var(--bg-tertiary); padding: 8px; border-radius: 4px;">${commit.sha}</div>
                </div>
                <div style="margin-bottom: 16px;">
                    <div style="color: var(--text-secondary); font-size: 12px; margin-bottom: 4px;">Message</div>
                    <div style="background: var(--bg-tertiary); padding: 8px; border-radius: 4px; white-space: pre-wrap;">${commit.message}</div>
                </div>
                <div style="margin-bottom: 16px;">
                    <div style="color: var(--text-secondary); font-size: 12px; margin-bottom: 4px;">Author</div>
                    <div>${commit.author} &lt;${commit.email}&gt;</div>
                </div>
                <div style="margin-bottom: 16px;">
                    <div style="color: var(--text-secondary); font-size: 12px; margin-bottom: 4px;">Date</div>
                    <div>${new Date(commit.date * 1000).toLocaleString()}</div>
                </div>
                ${commit.parent_shas.length > 0 ? `
                    <div style="margin-bottom: 16px;">
                        <div style="color: var(--text-secondary); font-size: 12px; margin-bottom: 4px;">Parents</div>
                        <div style="font-family: monospace; font-size: 11px;">
                            ${commit.parent_shas.map(sha => `<div>${sha}</div>`).join('')}
                        </div>
                    </div>
                ` : ''}
                ${commit.branches && commit.branches.length > 0 ? `
                    <div style="margin-bottom: 16px;">
                        <div style="color: var(--text-secondary); font-size: 12px; margin-bottom: 4px;">Branches</div>
                        <div>${commit.branches.map(b => `<span style="background: var(--accent-blue); color: white; padding: 2px 8px; border-radius: 3px; margin-right: 4px; font-size: 11px;">${b}</span>`).join('')}</div>
                    </div>
                ` : ''}
                ${commit.tags && commit.tags.length > 0 ? `
                    <div style="margin-bottom: 16px;">
                        <div style="color: var(--text-secondary); font-size: 12px; margin-bottom: 4px;">Tags</div>
                        <div>${commit.tags.map(t => `<span style="background: #f59e0b; color: white; padding: 2px 8px; border-radius: 3px; margin-right: 4px; font-size: 11px;">${t}</span>`).join('')}</div>
                    </div>
                ` : ''}
            </div>
        `;

        modal.className = 'commit-modal';
        modal.appendChild(content);
        document.body.appendChild(modal);

        modal.addEventListener('click', (e) => {
            if (e.target === modal) {
                modal.remove();
            }
        });
    }

    showError(message) {
        this.container.innerHTML = `
            <div style="padding: 20px; text-align: center; color: var(--text-danger);">
                <div style="font-size: 14px; font-weight: 500; margin-bottom: 8px;">Error</div>
                <div style="font-size: 12px;">${message}</div>
            </div>
        `;
    }
}

// Export for use in other scripts
if (typeof window !== 'undefined') {
    window.GitGraph = GitGraph;
}
