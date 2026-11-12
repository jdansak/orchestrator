import { Navigate, Route, Routes } from 'react-router-dom';
import { AuthProvider, useAuth } from './auth/AuthContext';
import LoginPage from './auth/LoginPage';
import Dashboard from './pages/Dashboard';
import TenantDetail from './pages/TenantDetail';
import Rollouts from './pages/Rollouts';
import Dns from './pages/Dns';
import Settings from './pages/Settings';

/** The application routes, gated on the session. */
function AuthenticatedRoutes() {
  const { session } = useAuth();

  if (!session) {
    return <Navigate to="/login" replace />;
  }

  return (
    <Routes>
      <Route path="/" element={<Dashboard />} />
      <Route path="/tenants/:name" element={<TenantDetail />} />
      <Route path="/rollouts" element={<Rollouts />} />
      <Route path="/dns" element={<Dns />} />
      <Route path="/settings" element={<Settings />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

/** The application root. */
export default function App() {
  return (
    <AuthProvider>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="*" element={<AuthenticatedRoutes />} />
      </Routes>
    </AuthProvider>
  );
}
