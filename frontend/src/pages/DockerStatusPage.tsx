import { useEffect, useState } from 'react'
import { getSshInfo, isLoggedIn } from '../api'
import type { SshInfo } from '../api'
import Spinner from '../components/Spinner'

export default function DockerStatusPage() {
  const [info, setInfo] = useState<SshInfo | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    getSshInfo()
      .then(setInfo)
      .catch(e => setError(e instanceof Error ? e.message : '載入失敗'))
  }, [])

  if (!isLoggedIn()) return <div className="error-box">請先登入</div>
  if (error) return <div className="error-box">{error}</div>
  if (!info) return <Spinner />

  return (
    <div style={{ maxWidth: 500 }}>
      <h2>容器狀態</h2>
      <p style={{ fontSize: 13, color: '#666', marginBottom: 20 }}>
        執行模式：{info.mode === 'docker' ? 'Docker' : 'Process'}
      </p>

      {info.mode === 'docker' ? (
        <section style={{ background: '#f5f5f5', borderRadius: 8, padding: 16 }}>
          <div style={{ marginBottom: 12 }}>
            <strong>Container：</strong>
            <code>{info.container}</code>
          </div>

          <div style={{ marginBottom: 12 }}>
            <strong>SSH 埠號：</strong>
            {info.ssh_port ? (
              <code>{info.ssh_port}</code>
            ) : <span style={{ color: '#999' }}>未分配</span>}
          </div>

          {info.ssh_password && (
            <div style={{ marginBottom: 12 }}>
              <strong>SSH 密碼：</strong>
              <code>{info.ssh_password}</code>
            </div>
          )}

          {info.ssh_port && (
            <div style={{ marginTop: 16, fontSize: 13, background: '#fff', borderRadius: 6, padding: 12 }}>
              <p style={{ fontWeight: 600, marginBottom: 8 }}>連線指令</p>
              <code style={{ display: 'block', padding: 8, background: '#222', color: '#0f0', borderRadius: 4 }}>
                ssh {info.ssh_password ? `-p ${info.ssh_port}` : `-p ${info.ssh_port}`} root@localhost
              </code>
              {info.ssh_password && (
                <p style={{ marginTop: 8, color: '#666' }}>
                  密碼：<strong>{info.ssh_password}</strong>
                </p>
              )}
            </div>
          )}
        </section>
      ) : (
        <p style={{ color: '#999' }}>Process 模式無容器資訊</p>
      )}
    </div>
  )
}
