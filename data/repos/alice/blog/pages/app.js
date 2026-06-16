document.querySelectorAll('.post-card').forEach(card => {
  card.addEventListener('click', function() {
    document.querySelectorAll('.post-card').forEach(c => c.style.borderColor = '#e4e4e7')
    this.style.borderColor = '#2563eb'
    this.style.boxShadow = '0 0 0 2px rgba(37,99,235,.15)'
  })
})

const h1 = document.querySelector('h1')
if (h1) {
  const colors = ['#18181b', '#2563eb', '#059669', '#d97706']
  let i = 0
  h1.addEventListener('click', () => {
    h1.style.color = colors[i % colors.length]
    i++
  })
}

const posts = document.querySelectorAll('.post-card')
const demoNotice = document.createElement('div')
demoNotice.style.cssText = 'background:#e0f2fe;border:1px solid #7dd3fc;border-radius:8px;padding:12px 16px;margin-bottom:24px;font-size:13px;color:#0369a1;'
demoNotice.textContent = 'Demo: click a post card to select it, click the title to cycle colors.'
document.querySelector('.hero')?.after(demoNotice)
