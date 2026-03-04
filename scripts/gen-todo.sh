#!/usr/bin/env bash
# Generate TODO.md from spool events using jq
# Usage: ./scripts/gen-todo.sh

set -euo pipefail

EVENTS_DIR=".spool/events"
OUTPUT="TODO.md"

if [ ! -d "$EVENTS_DIR" ]; then
    echo "No .spool/events directory"
    exit 0
fi

# Check for jq
if ! command -v jq &> /dev/null; then
    echo "jq is required but not installed"
    exit 1
fi

# Combine all jsonl files
cat "$EVENTS_DIR"/*.jsonl > /tmp/all_events.jsonl 2>/dev/null || true

if [ ! -s /tmp/all_events.jsonl ]; then
    echo "No events found"
    exit 0
fi

# Parse events and build markdown with jq
jq -rs '
  # Build streams map
  (map(select(.op == "create_stream")) |
    reduce .[] as $e ({}; .[$e.id] = {id: $e.id, name: $e.d.name})) as $streams |

  # Build tasks map
  reduce .[] as $e ({};
    if $e.op == "create" then
      .[$e.id] = {
        id: $e.id,
        title: $e.d.title,
        priority: ($e.d.priority // "p3"),
        stream: ($e.d.stream // ""),
        status: "open"
      }
    elif $e.op == "complete" then
      .[$e.id].status = "closed"
    elif $e.op == "update" then
      if $e.d.title then .[$e.id].title = $e.d.title else . end |
      if $e.d.priority then .[$e.id].priority = $e.d.priority else . end |
      if $e.d.stream then .[$e.id].stream = $e.d.stream else . end
    else . end
  ) |

  # Get open tasks grouped by stream
  to_entries | map(.value) | map(select(.status == "open")) |
  group_by(.stream) |

  # Sort groups by highest priority task
  sort_by(.[0].priority) |

  # Format as markdown
  ["# TODO", "", "_Auto-generated from spool. Do not edit manually._", ""] +
  (map(
    . as $tasks |
    ($tasks[0].stream) as $stream_id |
    ($streams[$stream_id].name // $stream_id // "Backlog") as $name |
    ["## \($name)", ""] +
    ($tasks | sort_by(.priority) | map("- [ ] \(.title) (\(.priority))")) +
    [""]
  ) | flatten) |
  join("\n")
' /tmp/all_events.jsonl > "$OUTPUT"

# Count tasks
open_count=$(jq -s '[.[] | select(.op == "create")] | length' /tmp/all_events.jsonl)
echo "Generated $OUTPUT ($open_count tasks tracked)"
