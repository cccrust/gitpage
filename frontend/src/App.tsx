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
import UserSettingsPage from './pages/UserSettingsPage'
import RepoSettingsPage from './pages/RepoSettingsPage'
import RepoSSHKeysPage from './pages/RepoSSHKeysPage'
import OrgList from './pages/OrgList'
import OrgCreate from './pages/OrgCreate'
import OrgDetail from './pages/OrgDetail'
import OrgSettings from './pages/OrgSettings'
import OrgMembers from './pages/OrgMembers'
import DockerStatusPage from './pages/DockerStatusPage'
import SettingsTokensPage from './pages/SettingsTokensPage'
import RepoSettingsCollaboratorsPage from './pages/RepoSettingsCollaboratorsPage'
import RepoSettingsSecretsPage from './pages/RepoSettingsSecretsPage'
import RepoSettingsBranchProtectionPage from './pages/RepoSettingsBranchProtectionPage'

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
          <Route path="/repo/:id/ssh" element={<RepoSSHKeysPage />} />
          <Route path="/u/:username" element={<UserProfilePage />} />
          <Route path="/settings" element={<UserSettingsPage />} />
          <Route path="/settings/tokens" element={<SettingsTokensPage />} />
          <Route path="/repo/:id/collaborators" element={<RepoSettingsCollaboratorsPage />} />
          <Route path="/repo/:id/secrets" element={<RepoSettingsSecretsPage />} />
          <Route path="/repo/:id/branch-protection" element={<RepoSettingsBranchProtectionPage />} />
          <Route path="/orgs" element={<OrgList />} />
          <Route path="/orgs/new" element={<OrgCreate />} />
          <Route path="/org/:name/members" element={<OrgMembers />} />
          <Route path="/org/:name/settings" element={<OrgSettings />} />
          <Route path="/org/:name" element={<OrgDetail />} />
          <Route path="/docker-status" element={<DockerStatusPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  )
}
