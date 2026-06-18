#!/bin/bash
source test/lib.sh
echo "=== 11-stars ==="
TK=$(register_user "star11"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "star-repo"); assert_gt "$RID" 0

# Star (response: { "starred": true, "stars_count": N })
STAR_RESP=$(api PUT "/api/repos/$RID/star" "$TK" "")
IS_STARRED=$(echo "$STAR_RESP" | extract "['starred']" 2>/dev/null)
assert_eq "$IS_STARRED" "true"

# Check starred status (response: { "starred": true/false })
CHECK=$(api GET "/api/repos/$RID/star" "$TK")
STARRED=$(echo "$CHECK" | extract "['starred']" 2>/dev/null)
assert_eq "$STARRED" "true"

# Unstar (response: { "starred": false, "stars_count": N })
UNSTAR=$(api DELETE "/api/repos/$RID/star" "$TK")
UNSTARRED=$(echo "$UNSTAR" | extract "['starred']" 2>/dev/null)
assert_eq "$UNSTARRED" "false"

# Watch (response: { "watching": true, "watch_count": N })
WATCH_RESP=$(api PUT "/api/repos/$RID/watch" "$TK" '{"watch_type":"watching"}')
WATCHING=$(echo "$WATCH_RESP" | extract "['watching']" 2>/dev/null)
assert_eq "$WATCHING" "true"

# Check watch status (response: { "watching": true/false, "watch_type": ... })
WATCHED=$(api GET "/api/repos/$RID/watch" "$TK")
WT=$(echo "$WATCHED" | extract "['watch_type']" 2>/dev/null)
assert_eq "$WT" "watching"

# Unwatch (response: { "watching": false, "watch_count": N })
UNWATCH=$(api DELETE "/api/repos/$RID/watch" "$TK")
NOT_WATCHING=$(echo "$UNWATCH" | extract "['watching']" 2>/dev/null)
assert_eq "$NOT_WATCHING" "false"

summary
