// Import WASM module
import init, { convert_extension_zip, analyze_extension_zip, analyze_keyboard_shortcuts } from './pkg/chrome2moz.js';

// State
let wasmModule = null;
let currentFile = null;
let analysisData = null;
let convertedData = null;
let shortcutData = null;
let selectedShortcuts = new Map(); // Map of original shortcut -> selected alternative

// Initialize WASM
async function initWasm() {
    try {
        wasmModule = await init();
        console.log('WASM module loaded successfully');
    } catch (error) {
        console.error('Failed to load WASM module:', error);
        showError('Failed to initialize converter. Please refresh the page.');
    }
}

// DOM Elements
const uploadBox = document.getElementById('uploadBox');
const fileInput = document.getElementById('fileInput');
const processingSection = document.getElementById('processingSection');
const analysisSection = document.getElementById('analysisSection');
const successSection = document.getElementById('successSection');
const errorSection = document.getElementById('errorSection');
const statusMessage = document.getElementById('statusMessage');
const statusDetail = document.getElementById('statusDetail');
const analysisResults = document.getElementById('analysisResults');
const convertBtn = document.getElementById('convertBtn');
const cancelBtn = document.getElementById('cancelBtn');
const downloadBtn = document.getElementById('downloadBtn');
const newConversionBtn = document.getElementById('newConversionBtn');
const retryBtn = document.getElementById('retryBtn');
const errorMessage = document.getElementById('errorMessage');
const downloadInfo = document.getElementById('downloadInfo');

// Event Listeners
fileInput.addEventListener('change', handleFileSelect);
uploadBox.addEventListener('dragover', handleDragOver);
uploadBox.addEventListener('dragleave', handleDragLeave);
uploadBox.addEventListener('drop', handleDrop);
uploadBox.addEventListener('click', (e) => {
    // Prevent default label behavior to avoid double-triggering
    if (e.target === uploadBox || e.target.closest('.icon') || e.target.closest('.text')) {
        // Let the label's default behavior handle opening file dialog
    }
});
convertBtn.addEventListener('click', handleConvert);
cancelBtn.addEventListener('click', resetUI);
downloadBtn.addEventListener('click', handleDownload);
newConversionBtn.addEventListener('click', resetUI);
retryBtn.addEventListener('click', resetUI);

// File Handling
function handleFileSelect(e) {
    const file = e.target.files[0];
    if (file) {
        processFile(file);
    }
}

function handleDragOver(e) {
    e.preventDefault();
    uploadBox.classList.add('drag-over');
}

function handleDragLeave(e) {
    e.preventDefault();
    uploadBox.classList.remove('drag-over');
}

function handleDrop(e) {
    e.preventDefault();
    uploadBox.classList.remove('drag-over');
    
    const file = e.dataTransfer.files[0];
    if (file) {
        processFile(file);
    }
}

// Process uploaded file
async function processFile(file) {
    if (!file.name.endsWith('.zip')) {
        showError('Please upload a ZIP file containing your Chrome extension.');
        return;
    }

    currentFile = file;
    showProcessing('Analyzing extension...');

    try {
        // Read file as array buffer
        const arrayBuffer = await file.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);

        // Analyze the extension
        statusDetail.textContent = 'Checking for incompatibilities...';
        const analysisJson = analyze_extension_zip(uint8Array);
        analysisData = JSON.parse(analysisJson);

        // Analyze keyboard shortcuts
        statusDetail.textContent = 'Checking keyboard shortcuts...';
        try {
            const shortcutJson = analyze_keyboard_shortcuts(uint8Array);
            shortcutData = JSON.parse(shortcutJson);
            console.log('Shortcut analysis:', shortcutData);
        } catch (error) {
            console.warn('Shortcut analysis failed:', error);
            shortcutData = null;
        }

        // Show analysis results
        showAnalysis(analysisData);
    } catch (error) {
        console.error('Analysis error:', error);
        showError(`Analysis failed: ${error.message || error}`);
    }
}

// Show analysis results
function showAnalysis(data) {
    hideAllSections();
    analysisSection.style.display = 'block';

    // Stats grid
    let html = '<div class="stats-grid">';
    html += `<div class="stat-card"><div class="label">Extension</div><div class="value">${data.extension_name}</div></div>`;
    html += `<div class="stat-card"><div class="label">Version</div><div class="value">${data.extension_version}</div></div>`;
    html += `<div class="stat-card"><div class="label">Manifest</div><div class="value">v${data.manifest_version}</div></div>`;
    html += `<div class="stat-card"><div class="label">Files</div><div class="value">${data.file_count}</div></div>`;
    html += `<div class="stat-card"><div class="label">Lines</div><div class="value">${data.line_count}</div></div>`;
    html += `<div class="stat-card"><div class="label">Issues</div><div class="value">${data.incompatibilities.length}</div></div>`;
    html += '</div>';

    // Group incompatibilities by category
    const categories = groupIncompatibilities(data.incompatibilities);

    if (data.incompatibilities.length === 0) {
        html += '<div style="text-align: center; padding: 2rem; color: var(--text-primary);">';
        html += '<h3>✓ No incompatibilities found</h3>';
        html += '<p>This extension should work well in Firefox.</p>';
        html += '</div>';
    } else {
        // Sort categories by severity (Blocker -> Major -> Minor -> Info)
        const severityOrder = { 'Blocker': 0, 'Major': 1, 'Minor': 2, 'Info': 3 };
        const sortedCategories = Object.keys(categories).sort((a, b) => {
            const issuesA = categories[a];
            const issuesB = categories[b];
            const maxSeverityA = Math.min(...issuesA.map(i => severityOrder[i.severity] ?? 4));
            const maxSeverityB = Math.min(...issuesB.map(i => severityOrder[i.severity] ?? 4));
            return maxSeverityA - maxSeverityB;
        });

        // Render each category as collapsible section
        sortedCategories.forEach((category, index) => {
            const issues = categories[category];
            if (issues.length > 0) {
                html += renderCollapsibleSection(category, issues, index === 0);
            }
        });
    }

    if (data.warnings && data.warnings.length > 0) {
        html += renderCollapsibleSection('Warnings', data.warnings.map(w => ({
            description: `<strong>${w.location || 'General'}:</strong> ${w.message}`,
            severity: 'Info'
        })), false);
    }

    // Add keyboard shortcut conflicts if any
    if (shortcutData && shortcutData.conflicts && shortcutData.conflicts.length > 0) {
        html += renderShortcutConflicts(shortcutData);
    }

    analysisResults.innerHTML = html;

    // Add event listeners for collapsible sections
    document.querySelectorAll('.section-header').forEach(header => {
        header.addEventListener('click', toggleSection);
    });
    
    // Set up shortcut selector event listeners
    setupShortcutSelectors();
}

// Group incompatibilities by TYPE OF SOLUTION (not by API)
function groupIncompatibilities(incompatibilities) {
    const categories = {
        // Simple conversions (just namespace changes)
        'Simple Namespace Conversions (chrome → browser)': [],
        
        // Callback to promise conversions
        'Callback-Style API Updates': [],
        
        // Manifest file changes
        'Manifest Modifications': [],
        
        // Chrome-only APIs grouped by solution type
        'APIs Using Compatibility Shims': [],
        'APIs Using Stubs (No-op)': [],
        'APIs with Workarounds': [],
        'Unsupported APIs (No Firefox Equivalent)': [],
        
        // Other
        'Permission Updates': [],
        'Configuration Changes': []
    };

    incompatibilities.forEach(issue => {
        const desc = issue.description.toLowerCase();
        const location = issue.location.toLowerCase();

        // Determine if this is JUST a namespace conversion or something more complex
        const isJustNamespace = (desc.includes('chrome namespace usage') || desc.includes('will be converted to browser'))
            && !desc.includes('callback')
            && !desc.includes('shim')
            && !desc.includes('stub')
            && !desc.includes('workaround')
            && !desc.includes('unsupported');

        if (isJustNamespace) {
            categories['Simple Namespace Conversions (chrome → browser)'].push(issue);
        }
        // Callback-style conversions (promise-based)
        else if (desc.includes('callback') || desc.includes('promise') || (desc.includes('async') && !location.includes('manifest'))) {
            categories['Callback-Style API Updates'].push(issue);
        }
        // Manifest changes
        else if (location.includes('manifest') || desc.includes('manifest.json') || desc.includes('browser_specific_settings')) {
            categories['Manifest Modifications'].push(issue);
        }
        // APIs using compatibility shims (partial/full implementations)
        else if (desc.includes('shim') ||
                 desc.includes('polyfill') ||
                 desc.includes('storage.session') ||
                 desc.includes('sidepanel') ||
                 desc.includes('action compat') ||
                 desc.includes('declarativenetrequest') ||
                 desc.includes('userscripts') ||
                 desc.includes('downloads') ||
                 desc.includes('notifications compat')) {
            categories['APIs Using Compatibility Shims'].push(issue);
        }
        // APIs using stubs (no-op implementations)
        else if (desc.includes('stub') ||
                 desc.includes('tabgroups') ||
                 desc.includes('privacy api') ||
                 (desc.includes('not supported') && desc.includes('stub'))) {
            categories['APIs Using Stubs (No-op)'].push(issue);
        }
        // APIs with workarounds (different approaches)
        else if (desc.includes('workaround') ||
                 desc.includes('alternative') ||
                 desc.includes('offscreen') ||
                 desc.includes('declarativecontent') ||
                 desc.includes('executescript') ||
                 desc.includes('content script') ||
                 desc.includes('worker') ||
                 (desc.includes('convert') && (desc.includes('web worker') || desc.includes('message passing')))) {
            categories['APIs with Workarounds'].push(issue);
        }
        // Completely unsupported APIs
        else if (desc.includes('unsupported') ||
                 desc.includes('not available') ||
                 desc.includes('no firefox equivalent') ||
                 (desc.includes('chrome-only') && !desc.includes('shim') && !desc.includes('stub'))) {
            categories['Unsupported APIs (No Firefox Equivalent)'].push(issue);
        }
        // Permission updates
        else if (desc.includes('permission') && !location.includes('manifest')) {
            categories['Permission Updates'].push(issue);
        }
        // Configuration changes
        else if (desc.includes('gecko.id') || desc.includes('extension id') || desc.includes('browser_specific')) {
            categories['Configuration Changes'].push(issue);
        }
    });

    // Filter out empty categories
    Object.keys(categories).forEach(key => {
        if (categories[key].length === 0) {
            delete categories[key];
        }
    });

    return categories;
}

// Render collapsible section with grouped common attributes
function renderCollapsibleSection(title, issues, isExpanded = true) {
    const sectionId = title.replace(/\s+/g, '-').toLowerCase();
    const expandedClass = isExpanded ? '' : 'collapsed';
    
    // Find common attributes across all issues
    const commonSeverity = issues.every(i => i.severity === issues[0].severity) ? issues[0].severity : null;
    const allAutoFixable = issues.every(i => i.auto_fixable);
    const commonSuggestion = issues.every(i => i.suggestion === issues[0].suggestion) ? issues[0].suggestion : null;
    
    // Calculate severity breakdown
    const severityBreakdown = {
        Blocker: issues.filter(i => i.severity === 'Blocker').length,
        Major: issues.filter(i => i.severity === 'Major').length,
        Minor: issues.filter(i => i.severity === 'Minor').length,
        Info: issues.filter(i => i.severity === 'Info').length
    };
    
    const autoFixable = issues.filter(i => i.auto_fixable).length;
    
    let html = '<div class="analysis-section">';
    html += `<div class="section-header ${expandedClass}" data-section="${sectionId}">`;
    html += '<div class="section-header-content">';
    html += `<h3>${title}</h3>`;
    html += '<div class="section-meta">';
    html += `<span class="section-count">${issues.length} change${issues.length !== 1 ? 's' : ''}</span>`;
    
    // Show severity breakdown
    const severities = [];
    if (severityBreakdown.Blocker > 0) severities.push(`${severityBreakdown.Blocker} blocker`);
    if (severityBreakdown.Major > 0) severities.push(`${severityBreakdown.Major} major`);
    if (severityBreakdown.Minor > 0) severities.push(`${severityBreakdown.Minor} minor`);
    if (severityBreakdown.Info > 0) severities.push(`${severityBreakdown.Info} info`);
    
    if (severities.length > 0) {
        html += `<span class="section-severity">• ${severities.join(', ')}</span>`;
    }
    
    if (autoFixable > 0) {
        html += `<span class="section-auto-fix">• ${autoFixable} auto-fixable</span>`;
    }
    
    html += '</div>';
    html += '</div>';
    html += '<span class="toggle-icon">▼</span>';
    html += '</div>';
    html += `<div class="section-content ${expandedClass}" id="${sectionId}">`;
    
    // Show common attributes once if they apply to all issues
    if (commonSeverity || allAutoFixable || commonSuggestion) {
        html += '<div class="common-attributes">';
        if (commonSeverity) {
            html += `<span class="severity-badge severity-${commonSeverity.toLowerCase()}">${commonSeverity}</span>`;
        }
        if (allAutoFixable) {
            html += '<span class="auto-fixable-badge">✓ Auto-fix</span>';
        }
        if (commonSuggestion) {
            html += `<div class="common-suggestion">💡 ${commonSuggestion}</div>`;
        }
        html += '</div>';
    }
    
    // Render issues in compact format
    html += '<div class="compact-issue-list">';
    issues.forEach(issue => {
        html += renderCompactIssue(issue, commonSeverity, allAutoFixable, commonSuggestion);
    });
    html += '</div>';
    
    html += '</div>';
    html += '</div>';
    
    return html;
}

// Render individual issue in compact format (no individual badges, just location + description)
function renderCompactIssue(issue, commonSeverity, commonAutoFix, commonSuggestion) {
    let html = '<div class="compact-issue">';
    
    // Only show these if NOT common (rare case where items differ within a category)
    const showSeverity = !commonSeverity;
    const showAutoFix = !commonAutoFix && issue.auto_fixable;
    const showSuggestion = !commonSuggestion && issue.suggestion;
    
    if (showSeverity || showAutoFix) {
        html += '<div class="issue-badges">';
        if (showSeverity) {
            html += `<span class="severity-badge severity-${issue.severity.toLowerCase()}">${issue.severity}</span>`;
        }
        if (showAutoFix) {
            html += '<span class="auto-fixable-badge">✓</span>';
        }
        html += '</div>';
    }
    
    html += `<div class="issue-location">${issue.location}</div>`;
    html += `<div class="issue-description">${issue.description}</div>`;

// Render keyboard shortcut conflicts with interactive selection
function renderShortcutConflicts(shortcutData) {
    if (!shortcutData.conflicts || shortcutData.conflicts.length === 0) {
        return '';
    }

    let html = '<div class="analysis-section shortcut-section">';
    html += '<div class="section-header" data-section="keyboard-shortcuts">';
    html += '<div class="section-header-content">';
    html += '<h3>⌨️ Keyboard Shortcut Conflicts</h3>';
    html += '<div class="section-meta">';
    html += `<span class="section-count">${shortcutData.conflicts.length} conflict${shortcutData.conflicts.length !== 1 ? 's' : ''}</span>`;
    html += '<span class="section-severity">• Action required</span>';
    html += '</div>';
    html += '</div>';
    html += '<span class="toggle-icon">▼</span>';
    html += '</div>';
    
    html += '<div class="section-content" id="keyboard-shortcuts">';
    html += '<div class="shortcut-conflicts-info">';
    html += '<p>⚠️ Your extension uses keyboard shortcuts that conflict with Firefox built-in shortcuts. ';
    html += 'Please select alternative shortcuts below. The converter will update your manifest automatically.</p>';
    html += '</div>';
    
    // Render each conflict with dropdown selector
    shortcutData.conflicts.forEach((conflict, index) => {
        html += renderShortcutConflict(conflict, index);
    });
    
    html += '</div>';
    html += '</div>';
    
    return html;
}

// Render individual shortcut conflict with selection dropdown
function renderShortcutConflict(conflict, index) {
    const conflictId = `shortcut-${index}`;
    const selectedAlt = selectedShortcuts.get(conflict.chrome_shortcut) || '';
    
    let html = '<div class="shortcut-conflict">';
    html += '<div class="conflict-header">';
    html += `<span class="conflict-shortcut">${escapeHtml(conflict.chrome_shortcut)}</span>`;
    html += '<span class="conflict-arrow">→</span>';
    html += `<span class="conflict-firefox">${escapeHtml(conflict.firefox_shortcut)}</span>`;
    html += '</div>';
    html += `<div class="conflict-description">Firefox uses this for: <strong>${escapeHtml(conflict.firefox_description)}</strong></div>`;
    
    // Alternative selector
    html += '<div class="shortcut-selector">';
    html += '<label for="' + conflictId + '">Select alternative:</label>';
    html += '<select id="' + conflictId + '" class="shortcut-dropdown" data-original="' + escapeHtml(conflict.chrome_shortcut) + '">';
    html += '<option value="">-- Choose a replacement --</option>';
    
    // Add suggested alternatives
    if (conflict.suggested_alternatives && conflict.suggested_alternatives.length > 0) {
        const alternatives = conflict.suggested_alternatives.slice(0, 15); // Limit display
        alternatives.forEach(alt => {
            const selected = selectedAlt === alt ? ' selected' : '';
            html += `<option value="${escapeHtml(alt)}"${selected}>${escapeHtml(alt)}</option>`;
        });
    }
    
    html += '<option value="custom">✏️ Enter custom shortcut...</option>';
    html += '</select>';
    
    // Custom input (initially hidden)
    html += '<input type="text" id="' + conflictId + '-custom" class="shortcut-custom-input" ';
    html += 'placeholder="e.g., Ctrl+Shift+X" style="display: none;">';
    
    html += '</div>';
    html += '</div>';
    
    return html;
}

// Escape HTML to prevent XSS
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Handle shortcut selection changes
function setupShortcutSelectors() {
    document.querySelectorAll('.shortcut-dropdown').forEach(select => {
        select.addEventListener('change', (e) => {
            const original = e.target.dataset.original;
            const value = e.target.value;
            const customInput = document.getElementById(e.target.id + '-custom');
            
            if (value === 'custom') {
                customInput.style.display = 'block';
                customInput.focus();
            } else {
                customInput.style.display = 'none';
                if (value) {
                    selectedShortcuts.set(original, value);
                } else {
                    selectedShortcuts.delete(original);
                }
            }
        });
    });
    
    // Handle custom input
    document.querySelectorAll('.shortcut-custom-input').forEach(input => {
        input.addEventListener('blur', (e) => {
            const selectId = e.target.id.replace('-custom', '');
            const select = document.getElementById(selectId);
            const original = select.dataset.original;
            const value = e.target.value.trim();
            
            if (value) {
                selectedShortcuts.set(original, value);
                // Add custom value to dropdown
                const option = document.createElement('option');
                option.value = value;
                option.textContent = value;
                option.selected = true;
                select.insertBefore(option, select.querySelector('option[value="custom"]'));
            }
        });
        
        input.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                e.target.blur();
            }
        });
    });
}
    
    if (showSuggestion) {
        html += `<div class="issue-suggestion">💡 ${issue.suggestion}</div>`;
    }
    
    html += '</div>';
    return html;
}

// Toggle section visibility
function toggleSection(e) {
    const header = e.currentTarget;
    const sectionId = header.dataset.section;
    const content = document.getElementById(sectionId);
    
    header.classList.toggle('collapsed');
    content.classList.toggle('collapsed');
}


// Handle conversion
async function handleConvert() {
    if (!currentFile) {
        showError('No file selected');
        return;
    }

    showProcessing('Converting extension...');

    try {
        // Get gecko.id input value
        const geckoIdInput = document.getElementById('geckoIdInput');
        const geckoId = geckoIdInput ? geckoIdInput.value.trim() : '';
        
        // Validate gecko.id format if provided
        if (geckoId && !validateGeckoId(geckoId)) {
            showError('Invalid extension ID format. Please use email format (e.g., extension@example.com)');
            return;
        }

        // Read file as array buffer
        const arrayBuffer = await currentFile.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);

        // Convert the extension
        statusDetail.textContent = 'Transforming files...';
        
        // TODO: Pass geckoId to WASM function when supported
        // For now, just convert normally
        convertedData = convert_extension_zip(uint8Array);

        // Show success
        showSuccess();
        
        // Log gecko.id if provided
        if (geckoId) {
            console.log('Custom Gecko ID will be used:', geckoId);
        }
    } catch (error) {
        console.error('Conversion error:', error);
        showError(`Conversion failed: ${error.message || error}`);
    }
}

// Validate gecko.id format
function validateGeckoId(id) {
    // Basic email format validation
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(id);
}

// Handle download
function handleDownload() {
    if (!convertedData) {
        showError('No converted data available');
        return;
    }

    try {
        // Get selected format
        const selectedFormat = document.querySelector('input[name="format"]:checked').value;
        const extension = selectedFormat === 'xpi' ? 'xpi' : 'zip';
        
        // Create blob and download
        const blob = new Blob([convertedData], { type: 'application/zip' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `${analysisData.extension_name.replace(/\s+/g, '-')}-firefox.${extension}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    } catch (error) {
        console.error('Download error:', error);
        showError('Failed to download file. Please try again.');
    }
}

// UI State Management
function hideAllSections() {
    document.querySelector('.upload-section').style.display = 'none';
    processingSection.style.display = 'none';
    analysisSection.style.display = 'none';
    successSection.style.display = 'none';
    errorSection.style.display = 'none';
}

function showProcessing(message) {
    hideAllSections();
    processingSection.style.display = 'block';
    statusMessage.textContent = message;
    statusDetail.textContent = 'This may take a moment...';
}

function showSuccess() {
    hideAllSections();
    successSection.style.display = 'block';
    
    if (analysisData) {
        const selectedFormat = document.querySelector('input[name="format"]:checked').value;
        const formatText = selectedFormat === 'xpi' ? 'XPI' : 'ZIP';
        downloadInfo.textContent = `${analysisData.extension_name} v${analysisData.extension_version} - Ready as ${formatText}`;
    }
}

function showError(message) {
    hideAllSections();
    errorSection.style.display = 'block';
    errorMessage.textContent = message;
}

function resetUI() {
    currentFile = null;
    analysisData = null;
    convertedData = null;
    fileInput.value = '';
    
    hideAllSections();
    document.querySelector('.upload-section').style.display = 'block';
}

// Initialize on load
initWasm();