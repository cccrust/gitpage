# Stars Handler — `stars.rs`

Handles star/unstar and watch/unwatch functionality for repositories. These are
social features that allow users to express interest in repos and receive
notifications.

## Star/Unstar Toggle

Stargazing is a toggle operation:

- `star_repo()` — Adds a star if not already starred. Returns the updated star
  count.
- `unstar_repo()` — Removes a star if starred. Returns the updated star count.
- `get_star_status()` — Returns `{ starred: true/false }` for the authenticated
  user.

The DB stores stars in a `stars` table (user_id + repo_id). Both operations verify
the repo exists before mutating. The `stars_count` in the repo record is updated
as a side effect (either by trigger or application logic).

## Watch/Unwatch with Watch Types

Watching is similar to starring but with a subscription model:

- `watch_repo()` — Subscribes with type `"participating"` (the only type currently
  implemented). This means the user will receive notifications for threads they
  participate in.
- `unwatch_repo()` — Unsubscribes.
- `get_watch_status()` — Returns `{ watching: true/false, watch_type: "..." }`.

The watch type is stored in the DB but the type-based notification filtering is
not yet implemented (all watchers get all notifications currently).

## Stargazer Lists and Counts

- `list_stargazers()` — Returns all users who have starred a repo, along with a
  `count`. No auth required (public).
- The count is also returned on every star/unstar mutation so the frontend can
  update the display without a separate request.
- Similarly for watchers, the `watch_count` is returned on watch/unwatch.

## User's Starred Repos

`list_user_stars()` — Returns all repos starred by a given user (by username).
This is used for the user's profile page "Stars" tab. The user is looked up first,
then `db.list_user_stars(user.id)` returns the list.

## Design Decisions

- **Simple toggle over idempotent API**: Unlike GitHub where starring a second
  time is a no-op, Gitpage's star/unstar are distinct operations. The caller
  should check status before toggling (or handle the case where the star already
  exists).
- **Counts on repos**: `stars_count` and `watch_count` are denormalized onto the
  `repositories` table for fast display. This avoids a COUNT query on every repo
  listing.
- **No social notifications**: Currently no "User X starred your repo" notifications
  or activity feed. The data is stored but not consumed beyond the star count.
- **Watch type is stored but not enforced**: The `"participating"` type is recorded
  but notification dispatch is not implemented.
