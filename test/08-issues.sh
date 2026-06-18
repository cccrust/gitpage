#!/bin/bash
source test/lib.sh
echo "=== 08-issues ==="
TK=$(register_user "iss08"); assert_not_empty "$TK"
RID=$(create_repo "$TK" "issues-repo"); assert_gt "$RID" 0

# Create label
LABEL=$(api POST "/api/repos/$RID/labels" "$TK" '{"name":"bug","color":"d73a4a"}')
assert_not_empty "$LABEL"

# List labels (response: { "labels": [...] })
LABELS=$(api GET "/api/repos/$RID/labels" "$TK")
LABEL_COUNT=$(echo "$LABELS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['labels']))" 2>/dev/null)
assert_gt "$LABEL_COUNT" 0

# Create issue (response: { "issue": { "issue": { "number": N, ... }, "author_username": "...", "labels": [] } })
ISSUE=$(api POST "/api/repos/$RID/issues" "$TK" '{"title":"Bug report","body":"something broke"}')
ISSUE_NUM=$(echo "$ISSUE" | extract "['issue']['issue']['number']")
assert_gt "$ISSUE_NUM" 0

# List issues (response: { "issues": [...] })
ISSUES=$(api GET "/api/repos/$RID/issues" "$TK")
ISSUE_COUNT=$(echo "$ISSUES" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['issues']))" 2>/dev/null)
assert_gt "$ISSUE_COUNT" 0

# Get single issue (response: { "issue": IssueWithAuthor })
ISSUE_GET=$(api GET "/api/repos/$RID/issues/$ISSUE_NUM" "$TK")
TITLE=$(echo "$ISSUE_GET" | extract "['issue']['issue']['title']")
assert_eq "$TITLE" "Bug report"

# Add comment (response: { "comment": IssueComment })
COMMENT=$(api POST "/api/repos/$RID/issues/$ISSUE_NUM/comments" "$TK" '{"body":"me too"}')
COMMENT_BODY=$(echo "$COMMENT" | extract "['comment']['body']")
assert_eq "$COMMENT_BODY" "me too"

# List comments (response: { "comments": [...] })
COMMENTS=$(api GET "/api/repos/$RID/issues/$ISSUE_NUM/comments" "$TK")
COMMENT_COUNT=$(echo "$COMMENTS" | python3 -c "import sys,json;d=json.load(sys.stdin);print(len(d['comments']))" 2>/dev/null)
assert_gt "$COMMENT_COUNT" 0

# Update issue (uses PUT, not PATCH)
api PUT "/api/repos/$RID/issues/$ISSUE_NUM" "$TK" '{"state":"closed"}' > /dev/null

# Check closed
CLOSED=$(api GET "/api/repos/$RID/issues/$ISSUE_NUM" "$TK")
STATE=$(echo "$CLOSED" | extract "['issue']['issue']['state']")
assert_eq "$STATE" "closed"

summary
