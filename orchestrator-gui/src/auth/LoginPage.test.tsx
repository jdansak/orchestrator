import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import LoginPage from '../auth/LoginPage';
import { AuthProvider } from '../auth/AuthContext';
import * as client from '../api/client';

vi.mock('../api/client', async () => {
  const actual = await vi.importActual<typeof import('../api/client')>('../api/client');
  return {
    ...actual,
    login: vi.fn(),
    logout: vi.fn(),
    readSession: vi.fn(() => null),
    writeSession: vi.fn(),
    clearSession: vi.fn(),
  };
});

function renderLogin() {
  return render(
    <MemoryRouter>
      <AuthProvider>
        <LoginPage />
      </AuthProvider>
    </MemoryRouter>,
  );
}

describe('LoginPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(client.readSession).mockReturnValue(null);
  });

  it('shows the error when the credentials are wrong', async () => {
    vi.mocked(client.login).mockRejectedValue(
      new client.ApiError(401, 'unauthorized', 'Unauthorized', 'Invalid credentials'),
    );

    renderLogin();

    await userEvent.type(screen.getByLabelText('Username'), 'admin');
    await userEvent.type(screen.getByLabelText('Password'), 'wrong');
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Invalid credentials');
    });
  });

  it('never distinguishes an unknown user from a wrong password', async () => {
    vi.mocked(client.login).mockRejectedValue(
      new client.ApiError(401, 'unauthorized', 'Unauthorized', 'Invalid credentials'),
    );

    renderLogin();

    await userEvent.type(screen.getByLabelText('Username'), 'nobody');
    await userEvent.type(screen.getByLabelText('Password'), 'wrong');
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Invalid credentials');
    });
  });

  it('does not report an error before a submission', () => {
    renderLogin();
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
