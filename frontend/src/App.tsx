import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import Layout from './components/Layout'
import LoginPage from './pages/LoginPage'
import RegisterPage from './pages/RegisterPage'
import Dashboard from './pages/Dashboard'
import NewRepoPage from './pages/NewRepoPage'
import RepoPage from './pages/RepoPage'
import FileViewPage from './pages/FileViewPage'
import FileExplorerPage from './pages/FileExplorerPage'
import FileEditorPage from './pages/FileEditorPage'
import DeployLogsPage from './pages/DeployLogsPage'
import DeployLogDetailPage from './pages/DeployLogDetailPage'
import CommitsPage from './pages/CommitsPage'
import PagesSettingsPage from './pages/PagesSettingsPage'
import AppSettingsPage from './pages/AppSettingsPage'
import UserProfilePage from './pages/UserProfilePage'
import RepoSettingsPage from './pages/RepoSettingsPage'

export default function App() {
  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/login" element={<LoginPage />} />
          <Route path="/register" element={<RegisterPage />} />
          <Route path="/new" element={<NewRepoPage />} />
          <Route path="/repo/:id" element={<RepoPage />} />
          <Route path="/repo/:id/files/edit" element={<FileEditorPage />} />
          <Route path="/repo/:id/files/new" element={<FileEditorPage />} />
          <Route path="/repo/:id/files" element={<FileExplorerPage />} />
          <Route path="/repo/:id/*" element={<FileViewPage />} />
          <Route path="/repo/:id/commits/:branch" element={<CommitsPage />} />
          <Route path="/repo/:id/pages" element={<PagesSettingsPage />} />
          <Route path="/repo/:id/app" element={<AppSettingsPage />} />
          <Route path="/repo/:id/deploys" element={<DeployLogsPage />} />
          <Route path="/repo/:id/deploys/:deployId" element={<DeployLogDetailPage />} />
          <Route path="/repo/:id/settings" element={<RepoSettingsPage />} />
          <Route path="/u/:username" element={<UserProfilePage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  )
}
