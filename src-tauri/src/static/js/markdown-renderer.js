// BerryCode Markdown Renderer
// Handles markdown rendering with syntax highlighting and copy buttons

// ========== Markdown Rendering ==========

/**
 * Render markdown to HTML with syntax highlighting
 */
function renderMarkdown(markdown) {
    if (!markdown) return '';

    // Simple markdown parser (lightweight, no external dependencies)
    let html = markdown;

    // Code blocks with syntax highlighting
    html = html.replace(/```(\w+)?\n([\s\S]*?)```/g, (match, language, code) => {
        const lang = language || 'plaintext';
        const highlighted = highlightCode(code.trim(), lang);
        const copyBtn = `<button class="code-copy-btn" onclick="copyCode(this)" title="コピー">📋</button>`;
        return `<div class="code-block-wrapper"><div class="code-block-header"><span class="code-language">${lang}</span>${copyBtn}</div><pre><code class="language-${lang}">${highlighted}</code></pre></div>`;
    });

    // Inline code
    html = html.replace(/`([^`]+)`/g, '<code class="inline-code">$1</code>');

    // Headers
    html = html.replace(/^### (.*$)/gim, '<h3>$1</h3>');
    html = html.replace(/^## (.*$)/gim, '<h2>$1</h2>');
    html = html.replace(/^# (.*$)/gim, '<h1>$1</h1>');

    // Bold
    html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    html = html.replace(/__([^_]+)__/g, '<strong>$1</strong>');

    // Italic
    html = html.replace(/\*([^*]+)\*/g, '<em>$1</em>');
    html = html.replace(/_([^_]+)_/g, '<em>$1</em>');

    // Links
    html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>');

    // Unordered lists
    html = html.replace(/^\* (.+)$/gim, '<li>$1</li>');
    html = html.replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>');

    // Ordered lists
    html = html.replace(/^\d+\. (.+)$/gim, '<li>$1</li>');

    // Line breaks
    html = html.replace(/\n\n/g, '</p><p>');
    html = html.replace(/\n/g, '<br>');

    // Wrap in paragraph
    if (!html.startsWith('<')) {
        html = '<p>' + html + '</p>';
    }

    return html;
}

/**
 * Simple syntax highlighter
 */
function highlightCode(code, language) {
    const escapeHtml = (text) => {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    };

    code = escapeHtml(code);

    // Language-specific highlighting
    if (language === 'javascript' || language === 'js' || language === 'typescript' || language === 'ts') {
        // Keywords
        code = code.replace(/\b(const|let|var|function|return|if|else|for|while|break|continue|class|extends|import|export|from|async|await|try|catch|finally|throw|new|this|super|static|get|set|typeof|instanceof)\b/g,
            '<span class="hl-keyword">$1</span>');

        // Strings
        code = code.replace(/(&quot;[^&]*&quot;|&#39;[^&]*&#39;|`[^`]*`)/g, '<span class="hl-string">$1</span>');

        // Comments
        code = code.replace(/(\/\/.*$)/gm, '<span class="hl-comment">$1</span>');
        code = code.replace(/(\/\*[\s\S]*?\*\/)/g, '<span class="hl-comment">$1</span>');

        // Numbers
        code = code.replace(/\b(\d+)\b/g, '<span class="hl-number">$1</span>');

        // Functions
        code = code.replace(/\b([a-zA-Z_]\w*)\s*(?=\()/g, '<span class="hl-function">$1</span>');

    } else if (language === 'python' || language === 'py') {
        // Keywords
        code = code.replace(/\b(def|class|import|from|return|if|elif|else|for|while|break|continue|pass|try|except|finally|raise|with|as|lambda|yield|global|nonlocal|assert|del|None|True|False|and|or|not|in|is)\b/g,
            '<span class="hl-keyword">$1</span>');

        // Strings
        code = code.replace(/(&quot;[^&]*&quot;|&#39;[^&]*&#39;)/g, '<span class="hl-string">$1</span>');

        // Comments
        code = code.replace(/(#.*$)/gm, '<span class="hl-comment">$1</span>');

        // Numbers
        code = code.replace(/\b(\d+)\b/g, '<span class="hl-number">$1</span>');

        // Functions
        code = code.replace(/\b([a-zA-Z_]\w*)\s*(?=\()/g, '<span class="hl-function">$1</span>');

    } else if (language === 'rust' || language === 'rs') {
        // Keywords
        code = code.replace(/\b(fn|let|mut|const|static|struct|enum|trait|impl|use|pub|mod|crate|super|self|if|else|match|loop|while|for|in|break|continue|return|async|await|move|ref|type|where|unsafe|extern|dyn)\b/g,
            '<span class="hl-keyword">$1</span>');

        // Strings
        code = code.replace(/(&quot;[^&]*&quot;)/g, '<span class="hl-string">$1</span>');

        // Comments
        code = code.replace(/(\/\/.*$)/gm, '<span class="hl-comment">$1</span>');
        code = code.replace(/(\/\*[\s\S]*?\*\/)/g, '<span class="hl-comment">$1</span>');

        // Numbers
        code = code.replace(/\b(\d+)\b/g, '<span class="hl-number">$1</span>');

        // Macros
        code = code.replace(/\b([a-zA-Z_]\w*!)/g, '<span class="hl-macro">$1</span>');

    } else if (language === 'html') {
        // Tags
        code = code.replace(/(&lt;\/?[a-zA-Z][\w-]*(?:\s[^&]*)?&gt;)/g, '<span class="hl-tag">$1</span>');

    } else if (language === 'css' || language === 'scss') {
        // Selectors
        code = code.replace(/^([.#]?[\w-]+)(?=\s*\{)/gm, '<span class="hl-selector">$1</span>');

        // Properties
        code = code.replace(/\b([\w-]+)(?=\s*:)/g, '<span class="hl-property">$1</span>');

        // Strings
        code = code.replace(/(&quot;[^&]*&quot;|&#39;[^&]*&#39;)/g, '<span class="hl-string">$1</span>');

        // Comments
        code = code.replace(/(\/\*[\s\S]*?\*\/)/g, '<span class="hl-comment">$1</span>');
    }

    return code;
}

/**
 * Copy code block to clipboard
 */
function copyCode(button) {
    const codeBlock = button.closest('.code-block-wrapper').querySelector('code');
    const code = codeBlock.textContent;

    navigator.clipboard.writeText(code).then(() => {
        const originalText = button.textContent;
        button.textContent = '✓';
        button.style.color = 'var(--accent-green, #10b981)';

        setTimeout(() => {
            button.textContent = originalText;
            button.style.color = '';
        }, 2000);
    }).catch(err => {
        console.error('Failed to copy code:', err);
        showToast('コピーに失敗しました', 'error');
    });
}

/**
 * Render a message with markdown
 */
function renderMessageContent(content) {
    // Check if content contains code blocks or markdown
    if (content.includes('```') || content.includes('**') || content.includes('##') || content.includes('[')) {
        return renderMarkdown(content);
    }

    // Plain text - just escape HTML
    const div = document.createElement('div');
    div.textContent = content;
    return div.innerHTML.replace(/\n/g, '<br>');
}

// ========== Export for use in template ==========
if (typeof window !== 'undefined') {
    window.renderMarkdown = renderMarkdown;
    window.renderMessageContent = renderMessageContent;
    window.copyCode = copyCode;
}
