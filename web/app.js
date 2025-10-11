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

    let html = '<div class="extension-info">';
    html += `<div class="info-item"><strong>Extension</strong><span>${data.extension_name}</span></div>`;
    html += `<div class="info-item"><strong>Version</strong><span>${data.extension_version}</span></div>`;
    html += `<div class="info-item"><strong>Manifest</strong><span>v${data.manifest_version}</span></div>`;
    html += `<div class="info-item"><strong>Files</strong><span>${data.file_count}</span></div>`;
    html += '</div>';

    // Group incompatibilities by severity
    const blockers = data.incompatibilities.filter(i => i.severity === 'Blocker');
    const majors = data.incompatibilities.filter(i => i.severity === 'Major');
    const minors = data.incompatibilities.filter(i => i.severity === 'Minor');
    const infos = data.incompatibilities.filter(i => i.severity === 'Info');

    if (data.incompatibilities.length === 0) {
        html += '<div class="analysis-item" style="border-left-color: #ffffff;">';
        html += '<h3>No incompatibilities found</h3>';
        html += '<p>This extension should work well in Firefox.</p>';
        html += '</div>';
    } else {
        html += `<h3>Found ${data.incompatibilities.length} incompatibilities:</h3>`;

        if (blockers.length > 0) {
            html += '<div class="analysis-item">';
            html += `<h4>${blockers.length} Blocker(s)</h4>`;
            blockers.forEach(issue => {
                html += renderIncompatibility(issue);
            });
            html += '</div>';
        }

        if (majors.length > 0) {
            html += '<div class="analysis-item">';
            html += `<h4>${majors.length} Major Issue(s)</h4>`;
            majors.forEach(issue => {
                html += renderIncompatibility(issue);
            });
            html += '</div>';
        }

        if (minors.length > 0) {
            html += '<div class="analysis-item">';
            html += `<h4>${minors.length} Minor Issue(s)</h4>`;
            minors.forEach(issue => {
                html += renderIncompatibility(issue);
            });
            html += '</div>';
        }

        if (infos.length > 0) {
            html += '<div class="analysis-item">';
            html += `<h4>${infos.length} Info Item(s)</h4>`;
            infos.forEach(issue => {
                html += renderIncompatibility(issue);
            });
            html += '</div>';
        }
    }

    if (data.warnings && data.warnings.length > 0) {
        html += '<div class="analysis-item">';
        html += '<h4>Warnings</h4>';
        data.warnings.forEach(warning => {
            html += `<p><strong>${warning.location || 'General'}:</strong> ${warning.message}</p>`;
        });
        html += '</div>';
    }

    analysisResults.innerHTML = html;
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
        // Read file as array buffer
        const arrayBuffer = await currentFile.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);

        // Convert the extension
        statusDetail.textContent = 'Transforming files...';
        convertedData = convert_extension_zip(uint8Array);

        // Show success
        showSuccess();
    } catch (error) {
        console.error('Conversion error:', error);
        showError(`Conversion failed: ${error.message || error}`);
    }
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