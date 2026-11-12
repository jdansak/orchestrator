import { useState, type FormEvent } from 'react';
import { Navigate, useNavigate } from 'react-router-dom';
import { ApiError } from '../api/client';
import { useAuth } from './AuthContext';

/** The login screen. */
export default function LoginPage() {
  const { session, login } = useAuth();
  const navigate = useNavigate();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (session) {
    return <Navigate to="/" replace />;
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await login(username, password);
      navigate('/', { replace: true });
    } catch (cause) {
      // The API returns the same message for an unknown user and a wrong password, and the
      // GUI must not invent a distinction either.
      setError(cause instanceof ApiError ? cause.message : 'Invalid credentials');
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="login">
      <h1>Orchestrator</h1>
      <form onSubmit={onSubmit}>
        <label htmlFor="username">Username</label>
        <input
          id="username"
          name="username"
          autoComplete="username"
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          required
        />
        <label htmlFor="password">Password</label>
        <input
          id="password"
          name="password"
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          required
        />
        {error ? (
          <p role="alert" className="error">
            {error}
          </p>
        ) : null}
        <button type="submit" disabled={busy}>
          {busy ? 'Signing in…' : 'Sign in'}
        </button>
      </form>
    </main>
  );
}
