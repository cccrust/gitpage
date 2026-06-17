import { useEffect, useState } from 'react'
import { useParams, Link } from 'react-router-dom'
import { getRepo, getBlob, listTree, type Repo, type TreeEntry } from '../api'
import MarkdownView from '../components/MarkdownView'
import Spinner from '../components/Spinner'

export default function FileViewPage() {
  const params = useParams()
  const id = params.id
  const rest = params['*'] || ''

  const [repo, setRepo] = useState<Repo | null>(null)
  const [content, setContent] = useState('')
  const [rendered, setRendered] = useState<string | null>(null)
  const [entries, setEntries] = useState<TreeEntry[]>([])
  const [isDir, setIsDir] = useState(false)
  const [loading, setLoading] = useState(true)
  const [err, setErr] = useState('')
  const [branch, setBranch] = useState('main')
  const [path, setPath] = useState('')

  useEffect(() => {
    if (!id || !rest) return

    const parts = rest.split('/')
    const b = parts[1] || 'main'
    const p = parts.slice(2).join('/')
    setBranch(b)
    setPath(p)

    const numId = parseInt(id)
    if (isNaN(numId)) { setErr('ID 無效'); setLoading(false); return }

    // Clear stale state before async ops
    setIsDir(false)
    setEntries([])
    setContent('')
    setRendered(null)
    setErr('')
    setLoading(true)

    getRepo(numId)
      .then(async r => {
        setRepo(r.repo)
        const uname = r.org_name || r.username
        const rname = r.repo.name

        try {
          const treeRes = await listTree(uname, rname, b, p)
          if (treeRes.entries && treeRes.entries.length > 0) {
            setIsDir(true)
            setEntries(treeRes.entries)
          }
        } catch {
          // not a directory
        }

        try {
          const blobRes = await getBlob(uname, rname, b, p)
          setContent(blobRes.content)
          if (blobRes.is_markdown && blobRes.rendered) {
            setRendered(blobRes.rendered)
          } else {
            setRendered(null)
          }
        } catch {
          // might be a dir
        }
      })
      .catch(e => setErr(e.message))
      .finally(() => setLoading(false))
  }, [id, rest])

  if (loading) return <Spinner />
  if (err) return <div className="error-box">{err}</div>
  if (!repo) return <div className="error-box">找不到</div>

  return (
    <div className="repo-page">
      <div className="repo-header">
        <div className="breadcrumb">
          <Link to="/">~</Link> / <Link to={`/repo/${repo.id}`}>{repo.name}</Link>
          {' / '}{path || '(root)'}
        </div>
      </div>

      {isDir ? (
        <div className="file-list">
          {entries.map(e => (
            <Link
              key={e.name}
              to={`/repo/${repo.id}/${e.is_dir ? 'tree' : 'blob'}/${branch}/${path ? path + '/' : ''}${e.name}`}
              className="file-entry"
            >
              <span className="icon">{e.is_dir ? '📁' : '📄'}</span>
              <span>{e.name}</span>
            </Link>
          ))}
        </div>
      ) : rendered ? (
        <MarkdownView html={rendered} />
      ) : (
        <div className="file-viewer">
          <div className="code-block">{content}</div>
        </div>
      )}
    </div>
  )
}
