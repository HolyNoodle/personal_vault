import { create } from 'zustand'


import { decodeToken, DecodedToken } from './jwtUtils';

export interface User {
  id: string;
  email: string;
  roles: string[];
}


interface AuthState {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  login: (token: string) => Promise<void>;
  logout: () => void;
  hasRole: (role: string) => boolean;
}

export const useAuthStore = create<AuthState>()((set, get) => ({
  user: null,
  token: null,
  isAuthenticated: false,
  login: async (token: string) => {
    const decoded = await decodeToken(token);
    if (decoded) {
      set({
        user: {
          id: decoded.id,
          email: decoded.email,
          roles: decoded.roles,
        },
        token,
        isAuthenticated: true,
      });
    }
  },
  logout: () => set({ user: null, token: null, isAuthenticated: false }),
  hasRole: (role: string) => {
    const user = get().user;
    return !!user && user.roles.includes(role);
  },
}));
