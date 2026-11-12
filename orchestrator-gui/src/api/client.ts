/**
 * The typed API client.
 *
 * Every request carries the bearer token from the session store. A `401` clears the session and
 * sends the user back to the login screen — the client never sees a stale token.
 *
 * The endpoint shapes are specified in `plan/08-api-reference.md`.
 */

const TOKEN_STORAGE_KEY = 'orchestrator.session';

/** A problem-details error, as defined by RFC 9457. */
export class ApiError extends Error {
  readonly status: number;
  readonly type: string;
  readonly errors: FieldError[];

  constructor(
    status: number,
    type: string,
    title: string,
    detail: string,
    errors: FieldError[] = [],
  ) {
    super(detail || title);
    this.name = 'ApiError';
    this.status = status;
    this.type = type;
    this.errors = errors;
  }
}

/** One field-level problem from a validation failure. */
export interface FieldError {
  field: string;
  message: string;
}

/** A session returned by `POST /api/login`. */
export interface Session {
  token: string;
  expires_at: string;
  user: { username: string; role: string };
}

/** Reads the stored session, if there is one. */
export function readSession(): Session | null {
  const raw = window.localStorage.getItem(TOKEN_STORAGE_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as Session;
  } catch {
    window.localStorage.removeItem(TOKEN_STORAGE_KEY);
    return null;
  }
}

/** Stores the session. */
export function writeSession(session: Session): void {
  window.localStorage.setItem(TOKEN_STORAGE_KEY, JSON.stringify(session));
}

/** Clears the session. */
export function clearSession(): void {
  window.localStorage.removeItem(TOKEN_STORAGE_KEY);
}

const BASE = '/api';

/**
 * Performs an authenticated request against the API.
 *
 * @throws {ApiError} for any non-2xx response.
 */
export async function request<T>(
  path: string,
  init: RequestInit = {},
  session: Session | null = readSession(),
): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set('accept', 'application/json');
  if (init.body && !headers.has('content-type')) {
    headers.set('content-type', 'application/json');
  }
  if (session) {
    headers.set('authorization', `Bearer ${session.token}`);
  }

  const response = await fetch(`${BASE}${path}`, { ...init, headers });

  if (response.status === 401) {
    clearSession();
    throw new ApiError(401, 'unauthorized', 'Unauthorized', 'The session has expired.');
  }

  if (!response.ok) {
    const problem = await readProblem(response);
    throw new ApiError(
      response.status,
      problem.type ?? 'about:blank',
      problem.title ?? 'Request failed',
      problem.detail ?? response.statusText,
      problem.errors ?? [],
    );
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

/** Reads a problem-details body, tolerating a non-JSON response. */
async function readProblem(response: Response): Promise<{
  type?: string;
  title?: string;
  detail?: string;
  errors?: FieldError[];
}> {
  try {
    return await response.json();
  } catch {
    return {};
  }
}

/** Logs in and stores the session. */
export async function login(username: string, password: string): Promise<Session> {
  const session = await request<Session>(
    '/login',
    { method: 'POST', body: JSON.stringify({ username, password }) },
    null,
  );
  writeSession(session);
  return session;
}

/** Logs out and clears the session. */
export async function logout(): Promise<void> {
  try {
    await request<void>('/logout', { method: 'POST' });
  } finally {
    clearSession();
  }
}
