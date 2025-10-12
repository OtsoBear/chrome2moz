# Chrome-Only API Implementation Status

> **Last Updated**: Based on [MDN Browser Compat Data](https://github.com/mdn/browser-compat-data) + additional APIs we track
>
> **Summary**: 179 Chrome-only APIs detected | 61 Implemented (34%) | 118 Not Implemented (66%)

This document tracks WebExtension APIs that are supported in Chrome but not in Firefox, along with their implementation status in this converter tool.

##  Quick Stats

| Category | Count | Percentage |
|----------|-------|------------|
| **Total Chrome-Only APIs** | 179 | 100% |
| ** Implemented** | 61 | 34% |
| ** Not Implemented** | 118 | 66% |

**Note**: Total includes 176 APIs from MDN data + 3 additional APIs we've implemented that aren't yet in MDN's dataset (`chrome.offscreen`, `chrome.declarativeContent`, `chrome.action.openPopup`).

##  Implementation Categories

###  Fully Implemented (58 APIs)

These APIs have automatic conversion support with compatibility shims or converters:

#### declarativeNetRequest (46 APIs) -  Complete Converter
- Full conversion from Chrome's DNR to Firefox's webRequest API
- **Implementation**: [`src/transformer/shims.rs:490-995`](src/transformer/shims.rs#L490)
- **Features**: Block, redirect, modifyHeaders, upgradeScheme actions
- **Status**: Production-ready with comprehensive rule conversion

<details>
<summary>View all declarativeNetRequest APIs (46)</summary>

- `declarativeNetRequest.getMatchedRules`
- `declarativeNetRequest.GETMATCHEDRULES_QUOTA_INTERVAL`
- `declarativeNetRequest.MAX_GETMATCHEDRULES_CALLS_PER_INTERVAL`
- `declarativeNetRequest.MAX_NUMBER_OF_UNSAFE_DYNAMIC_RULES`
- `declarativeNetRequest.MAX_NUMBER_OF_UNSAFE_SESSION_RULES`
- `declarativeNetRequest.onRuleMatchedDebug`
- `declarativeNetRequest.onRuleMatchedDebug.request`
- `declarativeNetRequest.onRuleMatchedDebug.request.documentId`
- `declarativeNetRequest.onRuleMatchedDebug.request.documentLifecycle`
- `declarativeNetRequest.onRuleMatchedDebug.request.frameId`
- `declarativeNetRequest.onRuleMatchedDebug.request.frameType`
- `declarativeNetRequest.onRuleMatchedDebug.request.initiator`
- `declarativeNetRequest.onRuleMatchedDebug.request.method`
- `declarativeNetRequest.onRuleMatchedDebug.request.parentDocumentId`
- `declarativeNetRequest.onRuleMatchedDebug.request.parentFrameId`
- `declarativeNetRequest.onRuleMatchedDebug.request.requestId`
- `declarativeNetRequest.onRuleMatchedDebug.request.tabId`
- `declarativeNetRequest.onRuleMatchedDebug.request.type`
- `declarativeNetRequest.onRuleMatchedDebug.request.url`
- `declarativeNetRequest.ResourceType.webbundle`
- `declarativeNetRequest.ResourceType.webtransport`
- `declarativeNetRequest.RuleCondition.domains`
- `declarativeNetRequest.RuleCondition.excludedDomains`
- `declarativeNetRequest.RuleCondition.excludedResponseHeaders`
- `declarativeNetRequest.RuleCondition.excludedResponseHeaders.excludedValues`
- `declarativeNetRequest.RuleCondition.excludedResponseHeaders.header`
- `declarativeNetRequest.RuleCondition.excludedResponseHeaders.values`
- `declarativeNetRequest.RuleCondition.responseHeaders`
- `declarativeNetRequest.RuleCondition.responseHeaders.excludedValues`
- `declarativeNetRequest.RuleCondition.responseHeaders.header`
- `declarativeNetRequest.RuleCondition.responseHeaders.values`
- `declarativeNetRequest.setExtensionActionOptions`
- `declarativeNetRequest.setExtensionActionOptions.options`
- `declarativeNetRequest.setExtensionActionOptions.options.tabUpdate`

</details>

#### sidePanel (10 APIs) -  Firefox Sidebar Adapter
- Maps Chrome's sidePanel to Firefox's sidebarAction API
- **Implementation**: [`src/transformer/shims.rs:410-487`](src/transformer/shims.rs#L410)
- **Status**: Functional with feature parity where possible

<details>
<summary>View all sidePanel APIs (10)</summary>

- `sidePanel` (base API)
- `sidePanel.getLayout`
- `sidePanel.getOptions`
- `sidePanel.getPanelBehavior`
- `sidePanel.GetPanelOptions`
- `sidePanel.onOpened`
- `sidePanel.open`
- `sidePanel.OpenOptions`
- `sidePanel.PanelBehavior`
- `sidePanel.PanelLayout`
- `sidePanel.PanelOpenedInfo`
- `sidePanel.PanelOptions`
- `sidePanel.setOptions`
- `sidePanel.setPanelBehavior`
- `sidePanel.Side`
- `sidePanel.SidePanel`

</details>

#### storage.session (1 API) -  In-Memory Polyfill
- Provides session storage using JavaScript Map
- **Implementation**: [`src/transformer/shims.rs:298-407`](src/transformer/shims.rs#L298)
- **Status**: Full API coverage with in-memory storage

- `storage.session.setAccessLevel`

#### Legacy APIs (3 APIs) -  Compatibility Wrappers
- **Implementation**: [`src/transformer/shims.rs:1055-1142`](src/transformer/shims.rs#L1055)
- **Status**: Automatic conversion to modern equivalents

<details>
<summary>View legacy APIs (3)</summary>

- `tabs.getAllInWindow` → Converted to `tabs.query()`
- `tabs.getSelected` → Converted to `tabs.query()`
- `downloads.acceptDanger` → Stub with warning
- `downloads.setShelfEnabled` → No-op stub

</details>

#### Other Implemented APIs (1 API)
- `runtime.getPackageDirectoryEntry` - Stub implementation ([`src/transformer/shims.rs:1145-1189`](src/transformer/shims.rs#L1145))
- `tabGroups.TabGroup.shared` - Part of tabGroups converter
- `userScripts.execute` - Part of userScripts shim

###  Not Yet Implemented (118 APIs)

These APIs are detected but don't have automatic conversion support yet:

#### action API (1)
- `action.UserSettingsChange`

#### bookmarks API (4)
- `bookmarks.onChildrenReordered`
- `bookmarks.onImportBegan`
- `bookmarks.onImportEnded`

#### browsingData API (11)
<details>
<summary>View all browsingData APIs (11)</summary>

- `browsingData.DataTypeSet.fileSystems`
- `browsingData.DataTypeSet.serverBoundCertificates`
- `browsingData.RemovalOptions.excludeOrigin`
- `browsingData.RemovalOptions.origin`
- `browsingData.RemovalOptions.originTypes`
- `browsingData.RemovalOptions.originTypes.extension`
- `browsingData.RemovalOptions.originTypes.protectedWeb`

</details>

#### devtools API (19)
<details>
<summary>View all devtools APIs (19)</summary>

- `devtools.inspectedWindow.eval.options.contextSecurityOrigin`
- `devtools.inspectedWindow.eval.options.frameURL`
- `devtools.inspectedWindow.eval.options.scriptExecutionContext`
- `devtools.inspectedWindow.eval.options.useContentScriptContext`
- `devtools.inspectedWindow.getResources`
- `devtools.inspectedWindow.onResourceAdded`
- `devtools.inspectedWindow.onResourceContentCommitted`
- `devtools.inspectedWindow.Resource`
- `devtools.inspectedWindow.Resource.getContent`
- `devtools.inspectedWindow.Resource.setContent`
- `devtools.inspectedWindow.Resource.url`
- `devtools.panels.Button`
- `devtools.panels.ExtensionPanel.createStatusBarButton`
- `devtools.panels.ExtensionPanel.onSearch`
- `devtools.panels.ExtensionSidebarPane.setHeight`
- `devtools.panels.openResource`
- `devtools.panels.setOpenResourceHandler`
- `devtools.panels.sources`
- `devtools.panels.SourcesPanel`

</details>

#### dom API (2)
- `dom` (base API - Chrome 88+)
- `dom.openOrClosedShadowRoot`

#### downloads API (3)
- `downloads.DownloadItem.danger`
- `downloads.DownloadItem.endTime`
- `downloads.DownloadQuery.endTime`
- `downloads.FilenameConflictAction.prompt`

#### events API (5)
- `events.Event.addRules`
- `events.Event.getRules`
- `events.Event.hasListeners`
- `events.Event.removeRules`
- `events.Rule`

#### extension API (Legacy, 5)
- `extension.getExtensionTabs`
- `extension.onRequest`
- `extension.onRequestExternal`
- `extension.sendRequest`
- `extension.setUpdateUrlData`

#### history API (1)
- `history.HistoryItem.typedCount`

#### idle API (2)
- `idle.onStateChanged.locked`
- `idle.queryState.locked`

#### management API (5)
- `management.ExtensionInfo.disabledReason`
- `management.ExtensionInfo.offlineEnabled`
- `management.ExtensionInfo.versionName`
- `management.getPermissionWarningsById`
- `management.getPermissionWarningsByManifest`
- `management.uninstall`

#### notifications API (11)
<details>
<summary>View all notifications APIs (11)</summary>

- `notifications.NotificationOptions.appIconMaskUrl`
- `notifications.NotificationOptions.buttons`
- `notifications.NotificationOptions.contextMessage`
- `notifications.NotificationOptions.eventTime`
- `notifications.NotificationOptions.imageUrl`
- `notifications.NotificationOptions.isClickable`
- `notifications.NotificationOptions.items`
- `notifications.NotificationOptions.priority`
- `notifications.NotificationOptions.progress`
- `notifications.NotificationOptions.requireInteraction`
- `notifications.onButtonClicked`
- `notifications.onClosed.byUser`
- `notifications.update`

</details>

#### privacy API (12)
<details>
<summary>View all privacy APIs (12)</summary>

- `privacy.services.alternateErrorPagesEnabled`
- `privacy.services.autofillAddressEnabled`
- `privacy.services.autofillCreditCardEnabled`
- `privacy.services.autofillEnabled`
- `privacy.services.safeBrowsingEnabled`
- `privacy.services.safeBrowsingExtendedReportingEnabled`
- `privacy.services.searchSuggestEnabled`
- `privacy.services.spellingServiceEnabled`
- `privacy.services.translationServiceEnabled`
- `privacy.websites.protectedContentEnabled`
- `privacy.websites.thirdPartyCookiesAllowed`

</details>

#### runtime API (8)
- `runtime.MessageSender.documentId`
- `runtime.MessageSender.documentLifecycle`
- `runtime.MessageSender.tlsChannelId`
- `runtime.onBrowserUpdateAvailable`
- `runtime.onInstalled.details.id`
- `runtime.onRestartRequired`
- `runtime.PlatformInfo.nacl_arch`
- `runtime.PlatformNaclArch`
- `runtime.requestUpdateCheck`
- `runtime.RequestUpdateCheckStatus`

#### storage API (1)
- `storage.StorageArea.setAccessLevel`

#### tabs API (12)
<details>
<summary>View all tabs APIs (12)</summary>

- `tabs.create.selected`
- `tabs.insertCSS.matchAboutBlank`
- `tabs.onActiveChanged`
- `tabs.onHighlightChanged`
- `tabs.onReplaced`
- `tabs.onSelectionChanged`
- `tabs.sendRequest`
- `tabs.setZoomSettings`
- `tabs.Tab.pendingUrl`
- `tabs.update.updateProperties_selected_parameter`

</details>

#### webNavigation API (5)
- `webNavigation.onCreatedNavigationTarget.sourceProcessId`
- `webNavigation.onErrorOccurred.error`
- `webNavigation.TransitionType.keyword_generated`
- `webNavigation.TransitionType.manual_subframe`
- `webNavigation.TransitionType.start_page`

#### webRequest API (1)
- `webRequest.onAuthRequired.asyncBlocking`

#### windows API (7)
- `windows.create.createData.focused`
- `windows.get.getInfo.windowTypes`
- `windows.getCurrent.getInfo.windowTypes`
- `windows.getLastFocused.getInfo.windowTypes`
- `windows.onBoundsChanged`
- `windows.Window.sessionId`
- `windows.WindowState.docked`

##  Missing from MDN Data

The following APIs are tracked in [`src/parser/javascript.rs`](src/parser/javascript.rs) but weren't found in MDN's dataset:

1. **`chrome.offscreen`** - Fully implemented with converter ([`src/transformer/offscreen_converter.rs`](src/transformer/offscreen_converter.rs))
2. **`chrome.declarativeContent`** - Fully implemented with converter ([`src/transformer/declarative_content_converter.rs`](src/transformer/declarative_content_converter.rs))
3. **`chrome.action.openPopup`** - Not yet implemented

These may be newer APIs or specific methods not yet documented in MDN's compatibility data.

##  Implementation Priority

Based on usage frequency and conversion feasibility:

### High Priority (Commonly Used)
1.  `declarativeNetRequest.*` - **COMPLETE**
2.  `storage.session.*` - **COMPLETE**
3.  `notifications.*` extended features - Partial support via shim
4.  `privacy.*` settings - Stub implementation exists

### Medium Priority (Moderate Usage)
1.  `tabs.*` extended features - Some legacy APIs covered
2.  `downloads.*` extended features - Basic stubs exist
3.  `management.*` - Not implemented
4.  `history.*` extended features - Not implemented

### Low Priority (Rarely Used/Legacy)
1.  `devtools.*` extended features - Developer tools specific
2.  `extension.*` legacy APIs - Deprecated
3.  `events.Rule.*` - Declarative events (legacy)
4.  `dom.*` - Newer Chrome-specific features

##  How to Check Latest Status

Run this command to fetch the latest Chrome-only APIs from MDN:

```bash
cargo run chrome-only-apis
```

This will:
- Fetch the latest browser compatibility data from GitHub
- Compare against implemented APIs in this tool
- Generate a detailed report with implementation status

##  Contributing

Want to help implement more Chrome-only APIs? Check out:

1. [`src/transformer/shims.rs`](src/transformer/shims.rs) - Add compatibility shims
2. [`src/parser/javascript.rs`](src/parser/javascript.rs) - Add API detection
3. [`ARCHITECTURE.md`](ARCHITECTURE.md) - Understand the conversion pipeline

##  References

- [MDN Browser Compat Data](https://github.com/mdn/browser-compat-data)
- [Chrome Extensions API](https://developer.chrome.com/docs/extensions/reference/)
- [Firefox WebExtensions API](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API)