import { useEffect, useState, useRef } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { getRepo, listWorkingTree, deleteFile, mkdir, writeFile, getStatus, commitRepo, isTextFile, type Repo, type FileEntry, type WorkingTreeChange } from '../api'

export default function FileExplorerPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()

  const [repo, setRepo] = useState<Repo | null>(null)
  const [entries, setEntries] = useState<FileEntry[]>([])
  const [currentPath, setCurrentPath] = useState('')
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [username, setUsername] = useState('')
  const [repoName, setRepoName] = useState('')

  const [pending, setPending] = useState(false)
  const [changes, setChanges] = useState<WorkingTreeChange[]>([])

  // Commit dialog
  const [showCommit, setShowCommit] = useState(false)
  const [commitMsg, setCommitMsg] = useState('')
  const [committing, setCommitting] = useState(false)

  // New folder dialog
  const [showNewDir, setShowNewDir] = useState(false)
  const [newDirName, setNewDirName] = useState('')

  // File upload
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [uploading, setUploading] = useState(false)

  const loadData = async () => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('Invalid ID'); setLoading(false); return }

    try {
      const r = await getRepo(numId)
      setRepo(r.repo)
      setUsername(r.username)
      setRepoName(r.repo.name)

      const [treeRes, statusRes] = await Promise.all([
        listWorkingTree(numId, currentPath || undefined),
        getStatus(numId),
      ])
      setEntries(treeRes.entries)
      setCurrentPath(treeRes.path)
      setPending(statusRes.pending)
      setChanges(statusRes.changes)
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Failed to load')
    }
    setLoading(false)
  }

  useEffect(() => {
    setLoading(true)
    loadData()
  }, [id])

  const navigateTo = (path: string) => {
    setLoading(true)
    setCurrentPath(path)
    setEntries([])
    setErr('')
    if (!id) return
    const numId = parseInt(id)
    listWorkingTree(numId, path || undefined)
      .then(res => {
        setEntries(res.entries)
        setCurrentPath(res.path)
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }

  const refresh = () => {
    setLoading(true)
    setErr('')
    loadData()
  }

  const handleDelete = async (name: string) => {
    if (!confirm(`Delete "${name}"?`)) return
    if (!id) return
    const path = currentPath ? `${currentPath}/${name}` : name
    try {
      await deleteFile(parseInt(id), path)
      refresh()
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : 'Delete failed')
    }
  }

  const handleNewDir = async () => {
    if (!newDirName.trim() || !id) return
    const path = currentPath ? `${currentPath}/${newDirName}` : newDirName
    try {
      await mkdir(parseInt(id), path)
      setShowNewDir(false)
      setNewDirName('')
      refresh()
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : 'Failed to create directory')
    }
  }

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files
    if (!files || !files.length || !id) return
    setUploading(true)
    const numId = parseInt(id)
    try {
      for (let i = 0; i < files.length; i++) {
        const file = files[i]
        const path = currentPath ? `${currentPath}/${file.name}` : file.name
        await writeFile(numId, path, file)
      }
      refresh()
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : 'Upload failed')
    }
    setUploading(false)
    if (fileInputRef.current) fileInputRef.current.value = ''
  }

  const handleCommit = async () => {
    if (!commitMsg.trim() || !id) return
    setCommitting(true)
    try {
      await commitRepo(parseInt(id), commitMsg)
      setShowCommit(false)
      setCommitMsg('')
      refresh()
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : 'Commit failed')
    }
    setCommitting(false)
  }

  const parts = currentPath ? currentPath.split('/').filter(Boolean) : []

  if (!id) return <div className="error-box">Missing repo ID</div>
  if (loading) return <div className="loading">Loading...</div>
  if (err) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">Repository not found</div>

  return (
    <div className="file-explorer-page">
      <div className="repo-header">
        <div className="breadcrumb">
          <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / <strong>files</strong>
        </div>
        <div className="explorer-actions">
          <button className="btn-sm" onClick={() => fileInputRef.current?.click()} disabled={uploading}>
            {uploading ? 'Uploading...' : 'Upload'}
          </button>
          <input ref={fileInputRef} type="file" multiple onChange={handleUpload} style={{ display: 'none' }} />
          <button className="btn-sm" onClick={() => setShowNewDir(true)}>New Folder</button>
          <button className="btn-sm" onClick={() => navigate(`/repo/${repo.id}/files/new?path=${encodeURIComponent(currentPath)}`)}>
            New File
          </button>
          <button className="btn" onClick={() => { setShowCommit(true); getStatus(parseInt(id!)).then(s => { setPending(s.pending); setChanges(s.changes) }) }}>
            Save Version
          </button>
        </div>
      </div>

      {/* Breadcrumb */}
      {currentPath && (
        <div className="branch-bar" style={{ display: 'flex', gap: 4, alignItems: 'center' }}>
          <a href="#" onClick={(e) => { e.preventDefault(); navigateTo('') }} style={{ fontSize: 13, color: '#7c7c7c' }}>root</a>
          {parts.map((part, i) => {
            const pathUpTo = parts.slice(0, i + 1).join('/')
            return (
              <span key={i}>
                <span style={{ color: '#7c7c7c', margin: '0 2px' }}>/</span>
                {i === parts.length - 1 ? (
                  <span style={{ fontSize: 13 }}>{part}</span>
                ) : (
                  <a href="#" onClick={(e) => { e.preventDefault(); navigateTo(pathUpTo) }} style={{ fontSize: 13, color: '#7c7c7c' }}>{part}</a>
                )}
              </span>
            )
          })}
        </div>
      )}

      {/* File list */}
      <div className="file-list">
        {currentPath && (
          <div className="file-entry" style={{ cursor: 'pointer' }} onClick={() => {
            const parent = parts.slice(0, -1).join('/')
            navigateTo(parent)
          }}>
            <span className="icon">📁</span>
            <span style={{ color: '#7c7c7c' }}>..</span>
          </div>
        )}
        {entries.length === 0 && !currentPath ? (
          <div className="empty-state">
            <p>No files yet. Upload or create a file to get started.</p>
          </div>
        ) : entries.length === 0 ? (
          <div className="empty-state">
            <p>Empty directory</p>
          </div>
        ) : (
          entries.map(e => (
            <div key={e.name} className="file-entry" style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '10px 0', borderBottom: 'var(--border)', fontSize: 14 }}>
              <span className="icon" style={{ fontSize: 16, width: 20, textAlign: 'center' }}>{e.is_dir ? '📁' : '📄'}</span>
              {e.is_dir ? (
                <a href="#" onClick={(ev) => { ev.preventDefault(); navigateTo(currentPath ? `${currentPath}/${e.name}` : e.name) }} style={{ flex: 1 }}>
                  {e.name}
                </a>
              ) : (
                <a href="#" onClick={(ev) => {
                  ev.preventDefault()
                  const fullPath = currentPath ? `${currentPath}/${e.name}` : e.name
                  if (isTextFile(e.name)) {
                    navigate(`/repo/${repo.id}/files/edit?path=${encodeURIComponent(fullPath)}`)
                  } else {
                    // Download binary
                    window.open(`/api/repos/${repo.id}/raw?path=${encodeURIComponent(fullPath)}`, '_blank')
                  }
                }} style={{ flex: 1 }}>
                  {e.name}
                </a>
              )}
              <span style={{ fontSize: 12, color: '#7c7c7c' }}>{e.updated_at}</span>
              {e.size != null && <span style={{ fontSize: 12, color: '#7c7c7c' }}>
                {e.size > 1024 ? `${(e.size / 1024).toFixed(1)} KB` : `${e.size} B`}
              </span>}
              <button className="btn-sm danger" onClick={() => handleDelete(e.name)} style={{ padding: '2px 8px', fontSize: 12 }}>Delete</button>
            </div>
          ))
        )}
      </div>

      {/* Pending changes bar */}
      {pending && (
        <div style={{ padding: '12px 0', display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: 'var(--border)' }}>
          <span style={{ fontSize: 13, color: '#7c7c7c' }}>
            {changes.length} pending change{changes.length !== 1 ? 's' : ''}
          </span>
          <span style={{ fontSize: 12, color: '#c00' }}>Unsaved version</span>
        </div>
      )}

      {/* New Folder Dialog */}
      {showNewDir && (
        <div className="modal-overlay" onClick={() => setShowNewDir(false)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>New Folder</h3>
            <input type="text" placeholder="Folder name" value={newDirName} onChange={e => setNewDirName(e.target.value)} autoFocus
              onKeyDown={e => { if (e.key === 'Enter') handleNewDir() }} />
            <div className="modal-actions">
              <button className="btn-sm" onClick={() => setShowNewDir(false)}>Cancel</button>
              <button className="btn" onClick={handleNewDir}>Create</button>
            </div>
          </div>
        </div>
      )}

      {/* Commit Dialog */}
      {showCommit && (
        <div className="modal-overlay" onClick={() => setShowCommit(false)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>Save Version</h3>
            {changes.length > 0 && (
              <div style={{ fontSize: 13, color: '#7c7c7c', marginBottom: 12, maxHeight: 150, overflow: 'auto' }}>
                {changes.map(c => (
                  <div key={c.path} style={{ padding: '2px 0' }}>
                    <span style={{ color: c.change_type === 'deleted' ? '#c00' : '#090', fontWeight: 600, marginRight: 8 }}>
                      {c.change_type === 'added' ? 'A' : c.change_type === 'modified' ? 'M' : 'D'}
                    </span>
                    {c.path}
                  </div>
                ))}
              </div>
            )}
            {changes.length === 0 && (
              <p style={{ fontSize: 13, color: '#7c7c7c', marginBottom: 12 }}>No changes to commit</p>
            )}
            <textarea
              placeholder="Commit message"
              value={commitMsg}
              onChange={e => setCommitMsg(e.target.value)}
              rows={3}
              autoFocus
              style={{ resize: 'vertical' }}
            />
            <div className="modal-actions">
              <button className="btn-sm" onClick={() => setShowCommit(false)}>Cancel</button>
              <button className="btn" onClick={handleCommit} disabled={committing || !commitMsg.trim() || changes.length === 0}>
                {committing ? 'Committing...' : 'Commit'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
