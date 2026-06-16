import { useEffect, useState } from 'react'
import { useParams, useSearchParams, useNavigate, useLocation, Link } from 'react-router-dom'
import { getRepo, getRawFile, writeFile, type Repo } from '../api'

export default function FileEditorPage() {
  const { id } = useParams<{ id: string }>()
  const [searchParams] = useSearchParams()
  const location = useLocation()
  const navigate = useNavigate()
  const filePath = searchParams.get('path') || ''

  const [repo, setRepo] = useState<Repo | null>(null)
  const [content, setContent] = useState('')
  const [originalContent, setOriginalContent] = useState('')
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [err, setErr] = useState('')
  const [msg, setMsg] = useState('')

  const isNew = location.pathname.endsWith('/files/new')

  useEffect(() => {
    if (!id) return
    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('Invalid ID'); setLoading(false); return }

    getRepo(numId)
      .then(async r => {
        setRepo(r.repo)
        if (!isNew && filePath) {
          const res = await getRawFile(numId, filePath)
          const text = await res.text()
          setContent(text)
          setOriginalContent(text)
        }
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id, filePath, isNew])

  const handleSave = async () => {
    if (!id || !filePath) return
    setSaving(true)
    setMsg('')
    setErr('')
    try {
      await writeFile(parseInt(id), filePath, content)
      setOriginalContent(content)
      setMsg('Saved')
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Save failed')
    }
    setSaving(false)
  }

  const handleNewFileSave = async () => {
    if (!id) return
    const name = prompt('File name (e.g. index.html):')
    if (!name) return
    const dir = searchParams.get('path') || ''
    const path = dir ? `${dir}/${name}` : name
    setSaving(true)
    setErr('')
    try {
      await writeFile(parseInt(id), path, content)
      navigate(`/repo/${id}/files/edit?path=${encodeURIComponent(path)}`)
      setMsg('Saved')
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Save failed')
    }
    setSaving(false)
  }

  if (!id) return <div className="error-box">Missing repo ID</div>
  if (loading) return <div className="loading">Loading...</div>
  if (err && !repo) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">Repository not found</div>

  return (
    <div className="file-editor-page">
      <div className="repo-header">
        <div className="breadcrumb">
          <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link> / <Link to={`/repo/${repo.id}/files`}>files</Link>
          {' / '}<strong>{isNew ? 'new file' : filePath}</strong>
        </div>
        <div className="explorer-actions" style={{ marginTop: 12 }}>
          {isNew ? (
            <button className="btn" onClick={handleNewFileSave} disabled={saving || !content.trim()}>
              {saving ? 'Creating...' : 'Create'}
            </button>
          ) : (
            <>
              <button className="btn" onClick={handleSave} disabled={saving || content === originalContent}>
                {saving ? 'Saving...' : 'Save'}
              </button>
              <Link to={`/repo/${repo.id}/files`} className="btn-sm">Back</Link>
            </>
          )}
        </div>
      </div>

      {msg && <p className="msg-ok">{msg}</p>}
      {err && <p className="msg-err">{err}</p>}

      <textarea
        className="editor-textarea"
        value={content}
        onChange={e => setContent(e.target.value)}
        placeholder={isNew ? 'Enter file content...' : 'Loading...'}
        spellCheck={false}
      />
    </div>
  )
}
