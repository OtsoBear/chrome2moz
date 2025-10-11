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

    // Calculate total lines of code (estimate based on file count)
    const estimatedLines = data.file_count * 50; // Rough estimate

    // Stats grid
    let html = '<div class="stats-grid">';
    html += `<div class="stat-card"><div class="label">Extension</div><div class="value">${data.extension_name}</div></div>`;
    html += `<div class="stat-card"><div class="label">Version</div><div class="value">${data.extension_version}</div></div>`;
    html += `<div class="stat-card"><div class="label">Manifest</div><div class="value">v${data.manifest_version}</div></div>`;
    html += `<div class="stat-card"><div class="label">Files</div><div class="value">${data.file_count}</div></div>`;
    html += `<div class="stat-card"><div class="label">Est. Lines</div><div class="value">~${estimatedLines}</div></div>`;
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
        'Chrome Namespace Conversions': [],
        'API Incompatibilities': [],
        'Manifest Issues': [],
        'Code Transformations': [],
        'Other Issues': []
    };

    incompatibilities.forEach(issue => {
        const desc = issue.description.toLowerCase();
        const location = issue.location.toLowerCase();

        if (desc.includes('chrome.') || desc.includes('namespace')) {
            categories['Chrome Namespace Conversions'].push(issue);
        } else if (location.includes('manifest')) {
            categories['Manifest Issues'].push(issue);
        } else if (desc.includes('api') || desc.includes('method') || desc.includes('property')) {
            categories['API Incompatibilities'].push(issue);
        } else if (desc.includes('transform') || desc.includes('convert') || desc.includes('replace')) {
            categories['Code Transformations'].push(issue);
        } else {
            categories['Other Issues'].push(issue);
        }
    });

    return categories;
}

// Render collapsible section
function renderCollapsibleSection(title, issues, isExpanded = true) {
    const sectionId = title.replace(/\s+/g, '-').toLowerCase();
    const expandedClass = isExpanded ? '' : 'collapsed';
    
    let html = '<div class="analysis-section">';
    html += `<div class="section-header ${expandedClass}" data-section="${sectionId}">`;
    html += '<h3>';
    html += `<span>${title}</span>`;
    html += `<span class="section-count">${issues.length}</span>`;
    html += '</h3>';
    html += '<span class="toggle-icon">▼</span>';
    html += '</div>';
    html += `<div class="section-content ${expandedClass}" id="${sectionId}">`;
    html += '<div class="incompatibility-list">';
    
    issues.forEach(issue => {
        html += renderIncompatibility(issue);
    });
    
    html += '</div>';
    html += '</div>';
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

// Render individual incompatibility
function renderIncompatibility(issue) {
    let html = '<div class="incompatibility">';
    html += '<div class="incompatibility-header">';
    html += `<span class="severity-badge">${issue.severity}</span>`;
    html += `<span class="location">${issue.location}</span>`;
    if (issue.auto_fixable) {
        html += '<span class="auto-fixable">Auto-fixable</span>';
    }
    html += '</div>';
    html += `<div class="description">${issue.description}</div>`;
    if (issue.suggestion) {
        html += `<div class="suggestion">${issue.suggestion}</div>`;
    }
    html += '</div>';
    return html;
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