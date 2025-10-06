#!/usr/bin/env python3
"""Fetch and list WebExtension APIs supported in Chrome but not Firefox.

This script fetches the browser-compat-data from GitHub's API without cloning
the entire repository. It lists the webextensions/api directory and downloads
JSON files concurrently for fast analysis.

Usage:
    python3 scripts/fetch_chrome_only_apis.py

Requirements:
    pip install aiohttp  # For async HTTP requests
"""

import asyncio
import json
import sys
from pathlib import Path
from typing import Any, Dict, Iterator, Tuple, List, Union

try:
    import aiohttp  # type: ignore
except ImportError:
    print("Error: aiohttp is required for async operations", file=sys.stderr)
    print("Install with: pip install aiohttp", file=sys.stderr)
    sys.exit(1)

# GitHub repository info
REPO_OWNER = "mdn"
REPO_NAME = "browser-compat-data"
BRANCH = "main"
API_PATH = "webextensions/api"

# GitHub API URLs
GITHUB_API_BASE = f"https://api.github.com/repos/{REPO_OWNER}/{REPO_NAME}"
GITHUB_RAW_BASE = f"https://raw.githubusercontent.com/{REPO_OWNER}/{REPO_NAME}/{BRANCH}"

PathParts = Tuple[str, ...]


async def fetch_json(session: aiohttp.ClientSession, url: str) -> Any:
    """Fetch JSON from URL asynchronously.
    
    Returns Any because different endpoints return different types:
    - GitHub contents API returns List[Dict]
    - Raw file content returns Dict
    """
    async with session.get(url) as response:
        response.raise_for_status()
        return await response.json(content_type=None)


async def list_api_files(session: aiohttp.ClientSession) -> List[str]:
    """List all JSON files in the webextensions/api directory."""
    url = f"{GITHUB_API_BASE}/contents/{API_PATH}?ref={BRANCH}"
    try:
        contents: List[Dict[str, Any]] = await fetch_json(session, url)
        return [
            item["name"]
            for item in contents
            if isinstance(item, dict) and item.get("name", "").endswith(".json")
        ]
    except Exception as e:
        print(f"Failed to list API files: {e}", file=sys.stderr)
        return []


async def fetch_api_file(session: aiohttp.ClientSession, filename: str) -> Tuple[str, Dict[str, Any]]:
    """Fetch a single API file from GitHub raw content."""
    url = f"{GITHUB_RAW_BASE}/{API_PATH}/{filename}"
    try:
        data = await fetch_json(session, url)
        return filename, data
    except Exception as e:
        print(f"Error fetching {filename}: {e}", file=sys.stderr)
        return filename, {}


def walk_support_entries(
    path: PathParts,
    node: Dict[str, Any],
    filename: str,
) -> Iterator[Tuple[PathParts, str, Dict[str, Any]]]:
    """Walk through compatibility entries in a data node."""
    compat = node.get("__compat")
    if isinstance(compat, dict):
        support = compat.get("support")
        if isinstance(support, dict):
            yield path, filename, support

    for key, child in node.items():
        if not isinstance(key, str) or key.startswith("__"):
            continue
        if isinstance(child, dict):
            yield from walk_support_entries(path + (key,), child, filename)
        elif isinstance(child, list):
            for index, item in enumerate(child):
                if isinstance(item, dict):
                    indexed_key = f"{key}[{index}]"
                    yield from walk_support_entries(path + (indexed_key,), item, filename)


def is_supported(entry: Any) -> bool:
    """Return True if the given support statement counts as supported."""
    if entry is None:
        return False
    if isinstance(entry, list):
        return any(is_supported(item) for item in entry)
    if isinstance(entry, bool):
        return entry
    if isinstance(entry, str):
        return entry.strip().lower() not in {"", "false", "no"}
    if isinstance(entry, dict):
        version_added = entry.get("version_added")
        if isinstance(version_added, str):
            return version_added.strip() not in {"", "false", "mirrored"}
        return bool(version_added)
    return False


def format_version(entry: Any) -> str:
    """Return a human-readable description of the browser support entry."""
    if entry is None:
        return "not supported"
    if isinstance(entry, list):
        formatted = [format_version(item) for item in entry]
        return "; ".join(formatted)
    if isinstance(entry, bool):
        return "supported" if entry else "not supported"
    if isinstance(entry, str):
        text = entry.strip() or "not supported"
        return text
    if isinstance(entry, dict):
        version_added = entry.get("version_added")
        if isinstance(version_added, str):
            version = version_added.strip()
            return version or "not supported"
        if version_added in (None, False):
            return "not supported"
        return str(version_added)
    return str(entry)


async def process_api_files(session: aiohttp.ClientSession, api_files: List[str]) -> List[Tuple[str, str, Any, Any]]:
    """Process all API files concurrently and return Chrome-only APIs."""
    results = []
    total = len(api_files)
    
    # Fetch all files concurrently
    print(f"Fetching {total} files concurrently...", file=sys.stderr)
    tasks = [fetch_api_file(session, filename) for filename in api_files]
    
    # Wait for all downloads to complete
    file_data = await asyncio.gather(*tasks, return_exceptions=True)
    
    # Process downloaded data
    print(f"Processing {total} files...", file=sys.stderr)
    processed = 0
    
    for item in file_data:
        # Handle exceptions from gather
        if isinstance(item, Exception):
            print(f"Skipping file due to error: {item}", file=sys.stderr)
            continue
        
        # Type guard: item is Tuple[str, Dict[str, Any]] at this point
        filename, data = item  # type: Tuple[str, Dict[str, Any]]
        processed += 1
        
        if processed % 10 == 0:
            print(f"Processed {processed}/{total} files...", file=sys.stderr)
        
        if not data:
            continue
            
        api_section = data.get("webextensions", {}).get("api", {})
        if not isinstance(api_section, dict):
            continue
        
        for api_name, api_data in api_section.items():
            if not isinstance(api_data, dict):
                continue
            
            for path_parts, source_file, support in walk_support_entries(
                (api_name,), api_data, filename
            ):
                chrome_info = support.get("chrome")
                firefox_info = support.get("firefox")
                
                if is_supported(chrome_info) and not is_supported(firefox_info):
                    feature_path = ".".join(path_parts)
                    results.append((feature_path, source_file, chrome_info, firefox_info))
    
    print(f"Completed processing all {processed} files", file=sys.stderr)
    return results


async def main_async() -> None:
    """Main async function."""
    async with aiohttp.ClientSession() as session:
        print("Fetching API file list from GitHub...", file=sys.stderr)
        api_files = await list_api_files(session)
        
        if not api_files:
            print("No API files found.", file=sys.stderr)
            return
        
        print(f"Found {len(api_files)} API files. Processing...", file=sys.stderr)
        results = await process_api_files(session, api_files)
        
        if not results:
            print("\nNo APIs found that are supported in Chrome but not in Firefox.")
            return
        
        print(f"\n{'='*80}")
        print("WebExtension APIs supported in Chrome but not Firefox:")
        print(f"{'='*80}\n")
        
        for api_name, source_file, chrome_info, firefox_info in sorted(results, key=lambda x: x[0].lower()):
            print(f"- {api_name}")
            print(f"    Source: {source_file}")
            print(f"    Chrome: {format_version(chrome_info)}")
            print(f"    Firefox: {format_version(firefox_info)}\n")
        
        print(f"\nTotal: {len(results)} Chrome-only APIs found")


def main() -> None:
    """Main entry point."""
    try:
        asyncio.run(main_async())
    except KeyboardInterrupt:
        print("\n\nInterrupted by user", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"\nError: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()