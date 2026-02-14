import { ReactNode } from 'react'
import { Link } from 'react-router-dom'
import { useAuthStore } from '../store/authStore'

interface LayoutProps {
  children: ReactNode
}

export function Layout({ children }: LayoutProps) {
  const { user, logout } = useAuthStore()

  return (
    <div style={{ display: 'flex', flexDirection: 'column', minHeight: '100vh' }}>
      <header style={{ 
        padding: '1rem 2rem', 
        background: '#1a1a1a', 
        borderBottom: '1px solid #333',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center'
      }}>
        <nav style={{ display: 'flex', gap: '1rem' }}>
          <Link to="/files" style={{ color: '#fff', textDecoration: 'none' }}>Files</Link>
          <Link to="/sessions" style={{ color: '#fff', textDecoration: 'none' }}>Sessions</Link>
        </nav>
        <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
          <span>{user?.email} ({user?.role})</span>
          <button onClick={logout}>Logout</button>
        </div>
      </header>
      <main style={{ flex: 1, padding: '2rem' }}>
        {children}
      </main>
    </div>
  )
}
