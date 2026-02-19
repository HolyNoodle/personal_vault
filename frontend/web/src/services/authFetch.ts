import { useAuthStore } from '../store/authStore';

export async function authFetch(input: RequestInfo, init: RequestInit = {}) {
  // Get token from zustand store
  // Note: This assumes useAuthStore.getState() is available outside React
  const token = useAuthStore.getState().token;
  const headers = new Headers(init.headers || {});
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }
  return fetch(input, { ...init, headers });
}
