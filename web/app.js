// Import WASM module
import init, { convert_extension_zip, analyze_extension_zip } from './pkg/chrome2moz.js';

// State
let wasmModule = null;
let currentFile = null;
let analysisData = null;
let convertedData = null;

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
    html += `<div class="stat-card"><div class="label">Lines</div><div class="value">${data.line_count.toLocaleString()}</div></div>`;
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
        // Render each category as collapsible section
        Object.keys(categories).forEach((category, index) => {
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

    analysisResults.innerHTML = html;

    // Add event listeners for collapsible sections
    document.querySelectorAll('.section-header').forEach(header => {
        header.addEventListener('click', toggleSection);
    });
}

// Group incompatibilities by category
function groupIncompatibilities(incompatibilities) {
    const categories = {
        'Namespace Conversions (chrome → browser)': [],
        'Callback-Style API Updates': [],
        'Manifest Changes': [],
        'Deprecated API Replacements': [],
        'Permission Updates': [],
        'Configuration Changes': [],
        'Code Transformations': [],
        'Other Issues': []
    };

    incompatibilities.forEach(issue => {
        const desc = issue.description.toLowerCase();
        const location = issue.location.toLowerCase();

        // Categorize based on description patterns
        if (desc.includes('chrome.') && desc.includes('browser.')) {
            categories['Namespace Conversions (chrome → browser)'].push(issue);
        } else if (desc.includes('callback') || desc.includes('promise') || desc.includes('async')) {
            categories['Callback-Style API Updates'].push(issue);
        } else if (location.includes('manifest') || desc.includes('manifest')) {
            categories['Manifest Changes'].push(issue);
        } else if (desc.includes('permission') || location.includes('permission')) {
            categories['Permission Updates'].push(issue);
        } else if (desc.includes('deprecated') || desc.includes('removed') || desc.includes('unsupported')) {
            categories['Deprecated API Replacements'].push(issue);
        } else if (desc.includes('gecko') || desc.includes('config') || desc.includes('browser_specific')) {
            categories['Configuration Changes'].push(issue);
        } else if (desc.includes('transform') || desc.includes('convert') || desc.includes('replace')) {
            categories['Code Transformations'].push(issue);
        } else {
            categories['Other Issues'].push(issue);
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

// Render individual issue in compact format
function renderCompactIssue(issue, commonSeverity, commonAutoFix, commonSuggestion) {
    let html = '<div class="compact-issue">';
    
    // Only show severity if not common
    if (!commonSeverity) {
        html += `<span class="severity-badge severity-${issue.severity.toLowerCase()}">${issue.severity}</span>`;
    }
    
    // Only show auto-fix if not all auto-fixable
    if (!commonAutoFix && issue.auto_fixable) {
        html += '<span class="auto-fixable-badge">✓</span>';
    }
    
    html += `<span class="issue-location">${issue.location}</span>`;
    html += `<span class="issue-description">${issue.description}</span>`;
    
    // Only show suggestion if not common
    if (!commonSuggestion && issue.suggestion) {
        html += `<span class="issue-suggestion">💡 ${issue.suggestion}</span>`;
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