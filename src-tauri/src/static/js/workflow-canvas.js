/**
 * BerryFlow - n8n-style Workflow Canvas
 * Drag & Drop visual workflow editor
 */

class WorkflowCanvas {
    constructor(containerId) {
        this.container = document.getElementById(containerId);
        this.canvas = this.createCanvas();
        this.ctx = this.canvas.getContext('2d');

        this.nodes = [];
        this.connections = [];
        this.selectedNode = null;
        this.draggingNode = null;
        this.connectingFrom = null;

        this.offsetX = 0;
        this.offsetY = 0;
        this.scale = 1.0;

        this.setupEventListeners();
        this.startRenderLoop();
    }

    createCanvas() {
        const canvas = document.createElement('canvas');
        canvas.id = 'workflow-canvas';
        canvas.style.width = '100%';
        canvas.style.height = '100%';
        this.container.appendChild(canvas);
        this.resizeCanvas(canvas);
        return canvas;
    }

    resizeCanvas(canvas) {
        const rect = this.container.getBoundingClientRect();
        canvas.width = rect.width;
        canvas.height = rect.height;
    }

    setupEventListeners() {
        // Mouse events
        this.canvas.addEventListener('mousedown', this.onMouseDown.bind(this));
        this.canvas.addEventListener('mousemove', this.onMouseMove.bind(this));
        this.canvas.addEventListener('mouseup', this.onMouseUp.bind(this));
        this.canvas.addEventListener('wheel', this.onWheel.bind(this));

        // Window resize
        window.addEventListener('resize', () => {
            this.resizeCanvas(this.canvas);
            this.render();
        });

        // Node palette drag
        this.setupNodePaletteDrag();
    }

    setupNodePaletteDrag() {
        const paletteItems = document.querySelectorAll('.node-palette-item');
        paletteItems.forEach(item => {
            item.addEventListener('dragstart', (e) => {
                e.dataTransfer.setData('nodeType', item.dataset.nodeType);
                e.dataTransfer.effectAllowed = 'copy';
            });
        });

        this.canvas.addEventListener('dragover', (e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = 'copy';
        });

        this.canvas.addEventListener('drop', (e) => {
            e.preventDefault();
            const nodeType = e.dataTransfer.getData('nodeType');
            const rect = this.canvas.getBoundingClientRect();
            const x = (e.clientX - rect.left - this.offsetX) / this.scale;
            const y = (e.clientY - rect.top - this.offsetY) / this.scale;
            this.addNode(nodeType, x, y);
        });
    }

    addNode(type, x, y) {
        const node = {
            id: `node-${Date.now()}`,
            type: type,
            name: this.getNodeDefaultName(type),
            x: x,
            y: y,
            width: 180,
            height: 80,
            inputs: this.getNodeInputs(type),
            outputs: this.getNodeOutputs(type),
            config: {},
        };
        this.nodes.push(node);
        this.selectedNode = node;
        this.render();
        this.onNodeAdded(node);
    }

    getNodeDefaultName(type) {
        const names = {
            'design': '設計',
            'implement': '実装',
            'test': 'テスト',
            'fix': '修正',
            'refactor': 'リファクタ',
            'doc': 'ドキュメント',
            'http': 'HTTPリクエスト',
            'script': 'スクリプト実行',
            'custom': 'カスタム',
        };
        return names[type] || type;
    }

    getNodeInputs(type) {
        // All nodes have one input except start nodes
        return type === 'start' ? [] : [{ name: 'input', label: '入力' }];
    }

    getNodeOutputs(type) {
        // Nodes can have success/failure outputs
        return [
            { name: 'success', label: '成功' },
            { name: 'failure', label: '失敗' },
        ];
    }

    onMouseDown(e) {
        const rect = this.canvas.getBoundingClientRect();
        const x = (e.clientX - rect.left - this.offsetX) / this.scale;
        const y = (e.clientY - rect.top - this.offsetY) / this.scale;

        // Check if clicking on output port (for connecting)
        const port = this.getPortAt(x, y);
        if (port && port.isOutput) {
            this.connectingFrom = port;
            return;
        }

        // Check if clicking on a node
        const node = this.getNodeAt(x, y);
        if (node) {
            this.selectedNode = node;
            this.draggingNode = node;
            this.dragOffsetX = x - node.x;
            this.dragOffsetY = y - node.y;
            this.onNodeSelected(node);
        } else {
            this.selectedNode = null;
            this.onNodeSelected(null);
        }

        this.render();
    }

    onMouseMove(e) {
        const rect = this.canvas.getBoundingClientRect();
        const x = (e.clientX - rect.left - this.offsetX) / this.scale;
        const y = (e.clientY - rect.top - this.offsetY) / this.scale;

        if (this.draggingNode) {
            this.draggingNode.x = x - this.dragOffsetX;
            this.draggingNode.y = y - this.dragOffsetY;
            this.render();
        } else if (this.connectingFrom) {
            this.tempConnectionX = x;
            this.tempConnectionY = y;
            this.render();
        }
    }

    onMouseUp(e) {
        if (this.connectingFrom) {
            const rect = this.canvas.getBoundingClientRect();
            const x = (e.clientX - rect.left - this.offsetX) / this.scale;
            const y = (e.clientY - rect.top - this.offsetY) / this.scale;

            const port = this.getPortAt(x, y);
            if (port && !port.isOutput && port.node.id !== this.connectingFrom.node.id) {
                // Create connection
                this.addConnection(this.connectingFrom.node.id, this.connectingFrom.port, port.node.id);
            }

            this.connectingFrom = null;
            this.tempConnectionX = null;
            this.tempConnectionY = null;
        }

        this.draggingNode = null;
        this.render();
    }

    onWheel(e) {
        e.preventDefault();
        const delta = e.deltaY > 0 ? 0.9 : 1.1;
        const newScale = this.scale * delta;

        if (newScale >= 0.5 && newScale <= 2.0) {
            this.scale = newScale;
            this.render();
        }
    }

    getNodeAt(x, y) {
        for (let i = this.nodes.length - 1; i >= 0; i--) {
            const node = this.nodes[i];
            if (x >= node.x && x <= node.x + node.width &&
                y >= node.y && y <= node.y + node.height) {
                return node;
            }
        }
        return null;
    }

    getPortAt(x, y) {
        const portRadius = 8;

        for (const node of this.nodes) {
            // Check output ports (right side)
            const outputX = node.x + node.width;
            const outputY1 = node.y + node.height / 3;
            const outputY2 = node.y + (2 * node.height) / 3;

            if (Math.hypot(x - outputX, y - outputY1) < portRadius) {
                return { node, port: 'success', isOutput: true };
            }
            if (Math.hypot(x - outputX, y - outputY2) < portRadius) {
                return { node, port: 'failure', isOutput: true };
            }

            // Check input port (left side)
            const inputX = node.x;
            const inputY = node.y + node.height / 2;

            if (Math.hypot(x - inputX, y - inputY) < portRadius) {
                return { node, port: 'input', isOutput: false };
            }
        }

        return null;
    }

    addConnection(fromNodeId, fromPort, toNodeId) {
        // Remove existing connections to the same input
        this.connections = this.connections.filter(
            conn => !(conn.toNodeId === toNodeId)
        );

        this.connections.push({
            fromNodeId,
            fromPort,
            toNodeId,
        });

        this.onConnectionAdded({ fromNodeId, fromPort, toNodeId });
        this.render();
    }

    removeConnection(conn) {
        const index = this.connections.indexOf(conn);
        if (index > -1) {
            this.connections.splice(index, 1);
            this.render();
        }
    }

    removeNode(node) {
        const index = this.nodes.indexOf(node);
        if (index > -1) {
            // Remove connections to/from this node
            this.connections = this.connections.filter(
                conn => conn.fromNodeId !== node.id && conn.toNodeId !== node.id
            );

            this.nodes.splice(index, 1);
            this.selectedNode = null;
            this.onNodeSelected(null);
            this.render();
        }
    }

    render() {
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

        // Draw grid
        this.drawGrid();

        this.ctx.save();
        this.ctx.translate(this.offsetX, this.offsetY);
        this.ctx.scale(this.scale, this.scale);

        // Draw connections
        this.drawConnections();

        // Draw temporary connection
        if (this.connectingFrom && this.tempConnectionX !== null) {
            this.drawTempConnection();
        }

        // Draw nodes
        this.drawNodes();

        this.ctx.restore();
    }

    drawGrid() {
        const gridSize = 20 * this.scale;
        this.ctx.strokeStyle = '#30363d';
        this.ctx.lineWidth = 1;

        for (let x = this.offsetX % gridSize; x < this.canvas.width; x += gridSize) {
            this.ctx.beginPath();
            this.ctx.moveTo(x, 0);
            this.ctx.lineTo(x, this.canvas.height);
            this.ctx.stroke();
        }

        for (let y = this.offsetY % gridSize; y < this.canvas.height; y += gridSize) {
            this.ctx.beginPath();
            this.ctx.moveTo(0, y);
            this.ctx.lineTo(this.canvas.width, y);
            this.ctx.stroke();
        }
    }

    drawNodes() {
        for (const node of this.nodes) {
            this.drawNode(node);
        }
    }

    drawNode(node) {
        const isSelected = this.selectedNode === node;

        // Node body
        this.ctx.fillStyle = this.getNodeColor(node.type);
        this.ctx.strokeStyle = isSelected ? '#3b82f6' : '#30363d';
        this.ctx.lineWidth = isSelected ? 3 : 2;

        this.roundRect(node.x, node.y, node.width, node.height, 8);
        this.ctx.fill();
        this.ctx.stroke();

        // Node icon/label
        this.ctx.fillStyle = '#ffffff';
        this.ctx.font = 'bold 14px sans-serif';
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText(node.name, node.x + node.width / 2, node.y + node.height / 2);

        // Draw ports
        this.drawPorts(node);
    }

    drawPorts(node) {
        const portRadius = 6;

        // Input port (left side)
        this.ctx.fillStyle = '#8b949e';
        this.ctx.beginPath();
        this.ctx.arc(node.x, node.y + node.height / 2, portRadius, 0, Math.PI * 2);
        this.ctx.fill();

        // Output ports (right side)
        // Success port (top)
        this.ctx.fillStyle = '#10b981';
        this.ctx.beginPath();
        this.ctx.arc(node.x + node.width, node.y + node.height / 3, portRadius, 0, Math.PI * 2);
        this.ctx.fill();

        // Failure port (bottom)
        this.ctx.fillStyle = '#ef4444';
        this.ctx.beginPath();
        this.ctx.arc(node.x + node.width, node.y + (2 * node.height) / 3, portRadius, 0, Math.PI * 2);
        this.ctx.fill();
    }

    drawConnections() {
        for (const conn of this.connections) {
            const fromNode = this.nodes.find(n => n.id === conn.fromNodeId);
            const toNode = this.nodes.find(n => n.id === conn.toNodeId);

            if (!fromNode || !toNode) continue;

            const fromX = fromNode.x + fromNode.width;
            const fromY = conn.fromPort === 'success'
                ? fromNode.y + fromNode.height / 3
                : fromNode.y + (2 * fromNode.height) / 3;

            const toX = toNode.x;
            const toY = toNode.y + toNode.height / 2;

            this.drawBezierConnection(fromX, fromY, toX, toY, conn.fromPort);
        }
    }

    drawTempConnection() {
        const node = this.connectingFrom.node;
        const fromX = node.x + node.width;
        const fromY = this.connectingFrom.port === 'success'
            ? node.y + node.height / 3
            : node.y + (2 * node.height) / 3;

        this.drawBezierConnection(fromX, fromY, this.tempConnectionX, this.tempConnectionY, this.connectingFrom.port);
    }

    drawBezierConnection(x1, y1, x2, y2, type) {
        const color = type === 'success' ? '#10b981' : '#ef4444';
        this.ctx.strokeStyle = color;
        this.ctx.lineWidth = 3;

        const cpOffset = Math.abs(x2 - x1) / 2;

        this.ctx.beginPath();
        this.ctx.moveTo(x1, y1);
        this.ctx.bezierCurveTo(
            x1 + cpOffset, y1,
            x2 - cpOffset, y2,
            x2, y2
        );
        this.ctx.stroke();

        // Draw arrow
        const arrowSize = 10;
        this.ctx.fillStyle = color;
        this.ctx.beginPath();
        this.ctx.moveTo(x2, y2);
        this.ctx.lineTo(x2 - arrowSize, y2 - arrowSize / 2);
        this.ctx.lineTo(x2 - arrowSize, y2 + arrowSize / 2);
        this.ctx.closePath();
        this.ctx.fill();
    }

    roundRect(x, y, width, height, radius) {
        this.ctx.beginPath();
        this.ctx.moveTo(x + radius, y);
        this.ctx.lineTo(x + width - radius, y);
        this.ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
        this.ctx.lineTo(x + width, y + height - radius);
        this.ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
        this.ctx.lineTo(x + radius, y + height);
        this.ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
        this.ctx.lineTo(x, y + radius);
        this.ctx.quadraticCurveTo(x, y, x + radius, y);
        this.ctx.closePath();
    }

    getNodeColor(type) {
        const colors = {
            'design': '#8b5cf6',
            'implement': '#3b82f6',
            'test': '#10b981',
            'fix': '#ef4444',
            'refactor': '#f59e0b',
            'doc': '#6366f1',
            'http': '#ec4899',
            'script': '#14b8a6',
            'custom': '#8b949e',
        };
        return colors[type] || '#1f2937';
    }

    startRenderLoop() {
        const loop = () => {
            requestAnimationFrame(loop);
        };
        loop();
    }

    // Event callbacks (override these)
    onNodeAdded(node) {}
    onNodeSelected(node) {}
    onConnectionAdded(connection) {}

    // Export/Import
    exportWorkflow() {
        return {
            nodes: this.nodes.map(n => ({
                id: n.id,
                type: n.type,
                name: n.name,
                x: n.x,
                y: n.y,
                config: n.config,
            })),
            connections: this.connections,
        };
    }

    importWorkflow(data) {
        this.nodes = data.nodes.map(n => ({
            ...n,
            width: 180,
            height: 80,
            inputs: this.getNodeInputs(n.type),
            outputs: this.getNodeOutputs(n.type),
        }));
        this.connections = data.connections;
        this.render();
    }

    clear() {
        this.nodes = [];
        this.connections = [];
        this.selectedNode = null;
        this.render();
    }
}
