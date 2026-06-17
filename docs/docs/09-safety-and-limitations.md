# Safety and Limitations

Shredator is destructive. Its job is to remove data, not move it to trash.

## What Shredator can provide

Shredator can provide best-effort deletion hygiene by overwriting a file through the filesystem API before removing it.

It is useful when:

- You control the path being deleted.
- The file is not already replicated elsewhere.
- You need a simple CLI tool with machine-readable confirmation.
- You want to reduce casual recovery risk.
- You are integrating file deletion into a larger controlled workflow.

## What Shredator cannot guarantee

Shredator cannot guarantee permanent deletion in all environments.

Important cases:

- SSD wear leveling.
- NVMe controller remapping.
- TRIM behavior.
- Filesystem journaling.
- Copy-on-write filesystems.
- Snapshots.
- Backups.
- Cloud sync.
- Antivirus quarantine.
- Search indexes.
- Thumbnail caches.
- Recent file lists.
- Temporary files created by editors.
- Memory/swap/pagefile remnants.
- Application-specific caches.
- Shell history containing file paths.
- Logs containing file names or contents.
- Previous copies of the file.

## SSDs and modern storage

On SSDs, overwriting a file through the filesystem does not necessarily overwrite the same physical flash cells. The device controller may write new blocks elsewhere and later erase old blocks internally.

For strong SSD sanitization, prefer:

- Full-disk encryption before sensitive data is written.
- Per-project encrypted containers.
- Vendor secure erase.
- Cryptographic erase/key destruction.
- OS/device-level sanitize commands.

Shredator is still useful as part of hygiene, but do not oversell it.

## Copy-on-write filesystems

On copy-on-write filesystems, an overwrite may allocate new blocks rather than modify old blocks in place. Old data can remain in snapshots or old extents.

Examples of environments where this matters:

- Btrfs.
- ZFS.
- APFS.
- ReFS in some configurations.
- Virtual disk images with snapshots.

## Journals and metadata

Even if file contents are overwritten, metadata can survive:

- Original filename.
- Directory entries.
- File size.
- Timestamps.
- Journal records.

`--zero-names` helps reduce filename exposure at the active directory entry level, but it is not a full metadata scrub.

## Backups and sync tools

If a file was ever backed up or synced, Shredator only affects the local target path. It does not delete copies from:

- Cloud storage.
- NAS snapshots.
- Backup software.
- Version history.
- Email attachments.
- Messaging apps.
- Temporary upload staging folders.

## Application traces

Applications often create hidden traces:

- Office autosaves.
- Photoshop scratch files.
- Video editor cache files.
- Browser downloads database.
- File explorer thumbnails.
- Recent documents.
- Search indexes.

Shredator does not know about these unless you explicitly target them.

## Permissions

Shredator needs permission to:

- Open the file for writing.
- Truncate the file.
- Rename the file when `--zero-names` is set.
- Delete the file.
- Remove directories.

Failure to satisfy any of those can produce warnings or errors.

## Read-only files

If a file is read-only or locked, Shredator may fail to open or remove it. The current implementation does not automatically change file permissions.

## Symlinks and special files

Treat symlinks, device files, sockets, FIFOs, and other special filesystem objects carefully. The current docs are written for regular files and directories. Wrapper code should preflight targets and decide whether symlinks are allowed.

Recommended wrapper behavior:

- Resolve/inspect file type before invoking Shredator.
- Reject symlinks by default unless the user explicitly allows them.
- Reject device files and special files.
- Apply path allowlists.

## Confirmation is not authorization

`--force` only skips Shredator's built-in confirmation. It does not mean the operation is safe.

Wrappers should implement their own authorization and confirmation before passing `--force`.

## Safer workflow for highly sensitive data

Best practice is prevention:

1. Store sensitive files only inside encrypted volumes or encrypted project folders.
2. Avoid creating unnecessary temporary copies.
3. Disable automatic cloud sync for sensitive work folders.
4. Use Shredator for cleanup of known paths.
5. Destroy encryption keys or use vendor/device secure erase when retiring media.

## Messaging recommendation

Do not promise users:

```text
This file is permanently unrecoverable.
```

Prefer:

```text
Shredator completed the requested overwrite/truncate/delete operation. This is best-effort deletion and does not guarantee recovery is impossible on all storage devices or from backups/snapshots.
```
