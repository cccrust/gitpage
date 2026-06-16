import { useEffect, useState, type FormEvent } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getRepo, listSshKeys, addSshKey, deleteSshKey, type Repo, type SshKey } from '../api'

export default function RepoSSHKeysPage() {
  const { id } = useParams<{ id: string }>()
  const [repo, setRepo] = useState<Repo | null>(null)
  const [keys, setKeys] = useState<SshKey[]>([])
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')

  const [keyName, setKeyName] = useState('')
  const [publicKey, setPublicKey] = useState('')
  const [adding, setAdding] = useState(false)
  const [addMsg, setAddMsg] = useState('')
  const [addErr, setAddErr] = useState('')

  const loadData = async (repoId: number) => {
    const [r, k] = await Promise.all([
      getRepo(repoId),
      listSshKeys(repoId),
    ])
    setRepo(r.repo)
    setKeys(k.ssh_keys)
  }

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('Invalid ID'); setLoading(false); return }

    loadData(numId)
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id])

  const handleAdd = async (e: FormEvent) => {
    e.preventDefault()
    if (!id) return
    setAdding(true)
    setAddMsg('')
    setAddErr('')
    try {
      await addSshKey(parseInt(id), keyName, publicKey)
      setKeyName('')
      setPublicKey('')
      setAddMsg('SSH key added.')
      await loadData(parseInt(id))
    } catch (e: unknown) {
      setAddErr(e instanceof Error ? e.message : 'Failed to add key')
    }
    setAdding(false)
  }

  const handleDelete = async (keyId: number) => {
    if (!id) return
    if (!confirm('Delete this SSH key?')) return
    try {
      await deleteSshKey(parseInt(id), keyId)
      await loadData(parseInt(id))
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Failed to delete key')
    }
  }

  if (!id) return <div className="error-box">Missing repo ID</div>
  if (loading) return <div className="loading">Loading...</div>
  if (err) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">Repository not found</div>

  return (
    <div className="settings-page" style={{ maxWidth: 600 }}>
      <div className="breadcrumb">
        <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / <strong>SSH Keys</strong>
      </div>
      <h2>SSH Keys</h2>
      <p className="hint">
        Add an SSH public key to access this repository's shell via SSH.
        When you connect, you will automatically enter the repository directory.
      </p>

      <form onSubmit={handleAdd}>
        <label>Key name</label>
        <input
          type="text"
          value={keyName}
          onChange={e => setKeyName(e.target.value)}
          placeholder="e.g. My MacBook Pro"
        />

        <label>SSH Public Key</label>
        <textarea
          value={publicKey}
          onChange={e => setPublicKey(e.target.value)}
          rows={3}
          placeholder="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINw..."
          required
        />

        {addMsg && <p className="msg-ok">{addMsg}</p>}
        {addErr && <p className="msg-err">{addErr}</p>}

        <button className="btn" type="submit" disabled={adding}>
          {adding ? 'Adding...' : 'Add Key'}
        </button>
      </form>

      <div style={{ marginTop: 24 }}>
        <h3>Existing Keys</h3>
        {keys.length === 0 ? (
          <p className="empty-state" style={{ textAlign: 'left', padding: '12px 0' }}>No SSH keys added yet.</p>
        ) : (
          <div className="ssh-key-list">
            {keys.map(k => (
              <div key={k.id} className="ssh-key-entry">
                <div className="ssh-key-info">
                  <span className="ssh-key-name">{k.name || '(unnamed)'}</span>
                  <span className="ssh-key-fingerprint">{k.public_key.slice(0, 50)}...</span>
                  <span className="ssh-key-date">{k.created_at}</span>
                </div>
                <button className="btn-sm-danger" onClick={() => handleDelete(k.id)}>Delete</button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="section" style={{ marginTop: 24 }}>
        <h3>Connecting via SSH</h3>
        <p className="hint">
          Once you have added a key, connect using:
        </p>
        <pre className="ssh-connection-info">ssh cccuser@localhost</pre>
        <p className="hint">
          You will automatically enter this repository's working directory.
        </p>
      </div>
    </div>
  )
}
