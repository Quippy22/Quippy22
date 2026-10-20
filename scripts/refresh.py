#!/usr/bin/env python3
"""Refresh assets/stats.txt with live data from the GitHub API.

    scripts/refresh.py            # everything, including lines of code
    scripts/refresh.py --no-loc   # skip the line count (no repo downloads)

Set GH_TOKEN to raise the API rate limit (60 requests/hour otherwise).
Anything that fails to fetch keeps its previous value.
"""

import json
import os
import sys
import tarfile
import urllib.error
import urllib.request
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STATS = ROOT / "assets" / "stats.txt"
TOKEN = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")

CODE_EXT = {
    ".rs", ".py", ".lua", ".sh", ".bash", ".zsh", ".fish", ".html", ".css",
    ".js", ".ts", ".jsx", ".tsx", ".c", ".h", ".cpp", ".go", ".java", ".sql",
    ".vim", ".nix",
}
COMMENT = {
    ".rs": "//", ".c": "//", ".h": "//", ".cpp": "//", ".go": "//",
    ".js": "//", ".ts": "//", ".jsx": "//", ".tsx": "//", ".java": "//",
    ".py": "#", ".sh": "#", ".bash": "#", ".zsh": "#", ".fish": "#", ".nix": "#",
    ".lua": "--", ".sql": "--", ".vim": '"',
}
# Extensions whose `/* ... */` blocks are worth stripping line by line.
BLOCK_COMMENT = {".rs", ".c", ".h", ".cpp", ".go", ".js", ".ts", ".jsx",
                 ".tsx", ".java", ".css"}
SKIP_DIRS = {
    ".git", "node_modules", "target", "dist", "build", "vendor", ".venv",
    "venv", "__pycache__", ".cache", "third_party",
}


def api(path):
    req = urllib.request.Request(
        f"https://api.github.com{path}",
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "quippy22-card",
        },
    )
    if TOKEN:
        req.add_header("Authorization", f"Bearer {TOKEN}")
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def read_stats():
    kv = {}
    if STATS.exists():
        for line in STATS.read_text().splitlines():
            if line.strip() and not line.startswith("#") and "=" in line:
                k, v = line.split("=", 1)
                kv[k.strip()] = v.strip()
    return kv


def write_stats(kv):
    lines = [
        "# GitHub data for card.svg — refresh with scripts/refresh.py",
        f"# generated: {date.today().isoformat()}",
    ]
    for k in ("user", "joined", "repos", "followers", "stars", "commits", "loc"):
        lines.append(f"{k}={kv.get(k, 0)}")
    langs = ",".join(f"{n}:{b}" for n, b in sorted(kv["languages"].items(), key=lambda x: -x[1]))
    lines.append(f"languages={langs}")
    STATS.write_text("\n".join(lines) + "\n")


def count_lines(url):
    """Code lines (no blanks, no whole-line comments) in a repo tarball.

    The tarball is streamed, so a repo with a large committed blob doesn't have
    to fit in memory.
    """
    total = 0
    req = urllib.request.Request(url, headers={"User-Agent": "quippy22-card"})
    with urllib.request.urlopen(req, timeout=120) as resp:
        with tarfile.open(fileobj=resp, mode="r|gz") as tar:
            for member in tar:
                if not member.isfile():
                    continue
                parts = Path(member.name).parts
                if any(p in SKIP_DIRS for p in parts):
                    continue
                ext = Path(member.name).suffix.lower()
                if ext not in CODE_EXT:
                    continue
                f = tar.extractfile(member)
                if f is None:
                    continue
                line_mark = COMMENT.get(ext)
                block = ext in BLOCK_COMMENT
                for raw in f.read().decode("utf8", "ignore").splitlines():
                    line = raw.strip()
                    if not line:
                        continue
                    if line_mark and line.startswith(line_mark):
                        continue
                    if block and line.startswith(("/*", "*", "*/")):
                        continue
                    total += 1
    return total


def main():
    no_loc = "--no-loc" in sys.argv
    kv = read_stats()
    user = kv.get("user", "Quippy22")
    old = dict(kv)

    profile = api(f"/users/{user}")
    kv["user"] = profile["login"]
    kv["joined"] = profile["created_at"][:10]
    kv["repos"] = profile["public_repos"]
    kv["followers"] = profile["followers"]

    repos = []
    page = 1
    while True:
        batch = api(f"/users/{user}/repos?per_page=100&page={page}&sort=updated")
        repos += batch
        if len(batch) < 100:
            break
        page += 1
    kv["repos"] = len(repos)
    kv["stars"] = sum(r["stargazers_count"] for r in repos)

    langs = {}
    try:
        for repo in repos:
            for name, size in api(f"/repos/{user}/{repo['name']}/languages").items():
                langs[name] = langs.get(name, 0) + size
        kv["languages"] = langs
    except (urllib.error.URLError, TimeoutError) as e:
        print(f"! languages: {e} (keeping {kv.get('languages', '?')})")

    try:
        search = api(f"/search/commits?q=author:{user}&per_page=1")
        kv["commits"] = search["total_count"]
    except (urllib.error.URLError, TimeoutError) as e:
        print(f"! commits: {e} (keeping {kv.get('commits', '?')})")

    if not no_loc and repos:
        loc = 0
        failed = 0
        for repo in repos:
            ref = repo["default_branch"]
            tar_url = f"https://codeload.github.com/{user}/{repo['name']}/tar.gz/refs/heads/{ref}"
            try:
                loc += count_lines(tar_url)
            except (urllib.error.URLError, TimeoutError, tarfile.TarError) as e:
                print(f"! {repo['name']}: {e} (skipped)")
                failed += 1
                continue
            print(f"  {repo['name']}: {loc} lines so far")
        if failed:
            print(f"! {failed} repo(s) unreadable — keeping previous lines of code")
        else:
            kv["loc"] = loc
    elif not repos:
        print("! lines of code: no repos found")

    write_stats(kv)
    for k in ("repos", "followers", "stars", "commits", "loc"):
        before = old.get(k, "?")
        after = kv.get(k)
        mark = " " if str(before) == str(after) else "*"
        print(f"{mark} {k:10} {before} -> {after}")
    print(f"wrote {STATS}")


if __name__ == "__main__":
    main()
