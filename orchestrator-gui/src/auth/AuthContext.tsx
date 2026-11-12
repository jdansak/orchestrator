import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react';
import { clearSession, login as apiLogin, logout as apiLogout, readSession, writeSession, type Session } from '../api/client';

interface AuthContextValue {
  session: Session | null;
  login: (username: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

/** Provides the session to the tree. */
export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<Session | null>(() => readSession());

  const login = useCallback(async (username: string, password: string) => {
    const next = await apiLogin(username, password);
    writeSession(next);
    setSession(next);
  }, []);

  const logout = useCallback(async () => {
    await apiLogout();
    clearSession();
    setSession(null);
  }, []);

  const value = useMemo(() => ({ session, login, logout }), [session, login, logout]);

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

/** Reads the auth context. */
export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used inside an AuthProvider');
  }
  return context;
}
