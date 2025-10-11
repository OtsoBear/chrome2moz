# Removed Features

This document tracks features and code that were removed from the chrome2moz converter to simplify the codebase and improve maintainability.

## Summary

**Commit Date**: 2025-10-11  
**Files Deleted**: 5  
**Lines Removed**: ~2,600  
**Reason**: Simplification and focus on core AST-based transformation

---

## Removed Files

### 1. AST_TRANSFORMER_PLAN.md (892 lines)
**Type**: Documentation  
**Reason**: Planning document no longer needed after implementation

This was a comprehensive implementation plan for the AST-based transformer. The document outlined:
- Phase 1-6 implementation roadmap
- Detailed technical specifications
- Code examples and patterns
- Success criteria and testing strategies

**Status**: The AST transformer has been implemented. The plan is archived in git history.

### 2. Test Output Files
**Files Removed**:
- `output2`
- `output3`
- `output4`
- `output6`

**Type**: Test artifacts  
**Reason**: Temporary debugging outputs, not needed in repository

### 3. test-callback-conversion/ (Submodule)
**Type**: Git submodule  
**Reason**: External test case no longer needed

This was a git submodule containing test cases for callback conversion. The functionality is now covered by internal tests.

---

## Removed Features

### 1. executeScript to Message Passing Transformation

**Location**: `src/transformer/javascript.rs` (lines removed: ~1,700)

#### What It Did
Converted Chrome's `scripting.executeScript` pattern to Firefox-compatible message-passing architecture:

```javascript
// Before (Chrome executeScript)
chrome.scripting.executeScript({
    target: { tabId: activeTab.id },
    function: (reqId) => {
        const result = myFunction(reqId);
        chrome.runtime.sendMessage({type: "RESULT", result});
    },
    args: [requestId]
});

// After (Message passing)
// Background.js:
browser.tabs.sendMessage(activeTab.id, {
    type: 'EXECUTE_SCRIPT_REQUEST_265',
    args: [requestId]
});

// Content.js (auto-generated):
browser.runtime.onMessage.addListener((request, sender, sendResponse) => {
    if (request.type === 'EXECUTE_SCRIPT_REQUEST_265') {
        const [reqId] = request.args;
        const result = myFunction(reqId);
        browser.runtime.sendMessage({type: "RESULT", result});
        return true;
    }
});
```

#### Why It Was Removed

1. **Complexity**: The transformation required complex AST analysis:
   - Function body extraction
   - Variable scope analysis
   - Background variable detection
   - Function reference lookup
   - Message listener generation

2. **Firefox Compatibility**: Firefox's `scripting.executeScript` works differently than Chrome's:
   - The isolation model is different
   - The generated message-passing code had compatibility issues
   - Required coordination between background and content scripts

3. **Better Alternative**: A simpler fix was implemented:
   - Chrome uses `function` parameter
   - Firefox uses `func` parameter
   - Simple parameter rename handles most cases

#### Impact
- Extensions using complex `executeScript` patterns may need manual conversion
- Simple script injection works with the new parameter rename approach
- Message-passing architecture should be implemented manually for complex cases

#### Code Removed
Key components removed:
- `ExecuteScriptCall` struct (function body/args/variables tracking)
- `parse_execute_script_calls()` - Detection and parsing
- `transform_execute_script_to_messages()` - Transformation logic
- `generate_content_script_listeners()` - Listener generation
- `find_background_variables_excluding_args()` - Scope analysis
- `lookup_function_body()` - Function reference resolution

### 2. Long Timer to Alarms Conversion

**Location**: `src/transformer/javascript.rs` (~200 lines)

#### What It Did
Converted long `setTimeout`/`setInterval` calls to `chrome.alarms` API:

```javascript
// Before
setTimeout(callback, 60000);  // > 30 seconds

// After
browser.alarms.create('converted_timeout_1', { delayInMinutes: 1 });
browser.alarms.onAlarm.addListener((alarm) => {
    if (alarm.name === 'converted_timeout_1') {
        callback();
    }
});
```

#### Why It Was Removed
1. **Edge Case**: Most extensions don't use long timers
2. **Complexity**: Required timer tracking and alarm listener generation
3. **Maintenance**: Added complexity for minimal benefit
4. **Better Handled Manually**: Extension developers can convert these manually if needed

#### Impact
- Extensions with long timers (>30s) need manual conversion
- Short timers continue to work as-is

#### Code Removed
- `TimerConversion` struct
- `convert_long_timers_to_alarms()`
- `generate_alarm_listeners()`
- Timer regex patterns

### 3. Content Script Listener Injection

**Location**: `src/transformer/mod.rs` (~40 lines)

#### What It Did
Automatically injected message listeners into `content.js` for executeScript compatibility.

#### Why It Was Removed
- Depended on the executeScript-to-message-passing transformation
- No longer needed with simplified executeScript approach
- Content scripts work as-is with proper browser API usage

---

## Simplified Architecture

### What Remains

The AST-based transformer now focuses on core transformations:

1. **API Namespace Conversion**: `chrome.*` → `browser.*`
2. **TypeScript Stripping**: Remove type annotations
3. **Scope Analysis**: Local variables vs global APIs
4. **executeScript Parameter Fix**: `function` → `func`
5. **Callback Transformations**: Basic promise conversion
6. **URL Replacement**: `chrome://` → `about:`

### Benefits of Simplification

1. **Maintainability**: 2,600 fewer lines to maintain
2. **Reliability**: Simpler code = fewer edge cases
3. **Performance**: Faster transformation
4. **Clarity**: Easier to understand and contribute to

### Migration Path

For extensions that relied on the removed features:

1. **executeScript**: Use simple function injection or implement message-passing manually
2. **Long Timers**: Convert to alarms manually if needed
3. **Complex Patterns**: Review the git history for reference implementations

---

## Git History

All removed code is preserved in git history:
- Last commit with full executeScript transformation: [previous commit]
- Last commit with timer conversion: [previous commit]
- AST_TRANSFORMER_PLAN.md: [previous commit]

To view removed code:
```bash
git log --all --full-history -- "AST_TRANSFORMER_PLAN.md"
git show [commit-hash]:src/transformer/javascript.rs
```

---

## Future Considerations

### May Be Re-added If:

1. **executeScript Transformation**: If a reliable, simpler implementation is found
2. **Timer Conversion**: If many users request this feature
3. **Advanced Patterns**: If community needs justify the complexity

### Alternative Approaches:

1. **Plugin System**: Allow optional transformations via plugins
2. **Configuration**: Let users opt-in to complex transformations
3. **Manual Mode**: Provide guidance instead of automatic transformation

---

**Last Updated**: October 2025  
**Version**: 0.1.0