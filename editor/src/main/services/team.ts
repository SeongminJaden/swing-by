import { ipcMain } from 'electron';
import crypto from 'crypto';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { getCurrentUser } from './auth';

// Types
type TeamRole = 'owner' | 'admin' | 'member';

interface TeamMember {
  userId: string;
  email: string;
  name: string;
  role: TeamRole;
  joinedAt: string;
}

interface Team {
  id: string;
  name: string;
  ownerId: string;
  members: TeamMember[];
  createdAt: string;
  updatedAt: string;
}

interface TeamsStore {
  teams: Team[];
}

// Paths
const DATA_DIR = path.join(os.homedir(), '.videplace');
const TEAMS_FILE = path.join(DATA_DIR, 'teams.json');

// Helpers
function ensureDataDir(): void {
  if (!fs.existsSync(DATA_DIR)) {
    fs.mkdirSync(DATA_DIR, { recursive: true });
  }
}

function readTeamsStore(): TeamsStore {
  ensureDataDir();
  if (!fs.existsSync(TEAMS_FILE)) {
    const empty: TeamsStore = { teams: [] };
    fs.writeFileSync(TEAMS_FILE, JSON.stringify(empty, null, 2), 'utf-8');
    return empty;
  }
  try {
    const raw = fs.readFileSync(TEAMS_FILE, 'utf-8');
    return JSON.parse(raw) as TeamsStore;
  } catch {
    return { teams: [] };
  }
}

function writeTeamsStore(store: TeamsStore): void {
  ensureDataDir();
  fs.writeFileSync(TEAMS_FILE, JSON.stringify(store, null, 2), 'utf-8');
}

function generateId(): string {
  return crypto.randomUUID();
}

// Read the users store to look up users by email
function readUsersStore(): { users: Array<{ id: string; email: string; name: string }> } {
  const usersFile = path.join(DATA_DIR, 'users.json');
  if (!fs.existsSync(usersFile)) {
    return { users: [] };
  }
  try {
    const raw = fs.readFileSync(usersFile, 'utf-8');
    return JSON.parse(raw);
  } catch {
    return { users: [] };
  }
}

export function registerTeamHandlers(): void {
  ipcMain.handle('team:create', async (_event, name: string) => {
    try {
      const user = getCurrentUser();
      if (!user) {
        return { success: false, error: 'Not logged in' };
      }

      if (!name || name.trim().length === 0) {
        return { success: false, error: 'Team name is required' };
      }

      const store = readTeamsStore();
      const now = new Date().toISOString();

      const newTeam: Team = {
        id: generateId(),
        name: name.trim(),
        ownerId: user.id,
        members: [
          {
            userId: user.id,
            email: user.email,
            name: user.name,
            role: 'owner',
            joinedAt: now,
          },
        ],
        createdAt: now,
        updatedAt: now,
      };

      store.teams.push(newTeam);
      writeTeamsStore(store);

      return {
        success: true,
        team: {
          id: newTeam.id,
          name: newTeam.name,
          ownerId: newTeam.ownerId,
          members: newTeam.members,
        },
      };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  ipcMain.handle('team:invite', async (_event, teamId: string, email: string) => {
    try {
      const user = getCurrentUser();
      if (!user) {
        return { success: false, error: 'Not logged in' };
      }

      if (!teamId || !email) {
        return { success: false, error: 'Team ID and email are required' };
      }

      const store = readTeamsStore();
      const teamIndex = store.teams.findIndex((t) => t.id === teamId);

      if (teamIndex === -1) {
        return { success: false, error: 'Team not found' };
      }

      const team = store.teams[teamIndex];

      // Only owner or admin can invite
      const callerMember = team.members.find((m) => m.userId === user.id);
      if (!callerMember || (callerMember.role !== 'owner' && callerMember.role !== 'admin')) {
        return { success: false, error: 'Permission denied: only owners and admins can invite' };
      }

      // Check if already a member
      if (team.members.find((m) => m.email === email)) {
        return { success: false, error: 'User is already a team member' };
      }

      // Look up invited user in the users store
      const usersStore = readUsersStore();
      const invitedUser = usersStore.users.find((u) => u.email === email);

      const newMember: TeamMember = {
        userId: invitedUser?.id || '',
        email,
        name: invitedUser?.name || email,
        role: 'member',
        joinedAt: new Date().toISOString(),
      };

      store.teams[teamIndex].members.push(newMember);
      store.teams[teamIndex].updatedAt = new Date().toISOString();
      writeTeamsStore(store);

      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  ipcMain.handle('team:getMyTeams', async () => {
    try {
      const user = getCurrentUser();
      if (!user) {
        return { success: false, error: 'Not logged in' };
      }

      const store = readTeamsStore();
      const myTeams = store.teams
        .filter((t) => t.members.some((m) => m.userId === user.id))
        .map((t) => {
          const member = t.members.find((m) => m.userId === user.id)!;
          return {
            id: t.id,
            name: t.name,
            role: member.role,
            memberCount: t.members.length,
          };
        });

      return myTeams;
    } catch (err: any) {
      return [];
    }
  });

  ipcMain.handle('team:getMembers', async (_event, teamId: string) => {
    try {
      const user = getCurrentUser();
      if (!user) {
        return { success: false, error: 'Not logged in' };
      }

      const store = readTeamsStore();
      const team = store.teams.find((t) => t.id === teamId);

      if (!team) {
        return { success: false, error: 'Team not found' };
      }

      // Must be a member to see members
      if (!team.members.some((m) => m.userId === user.id)) {
        return { success: false, error: 'Permission denied' };
      }

      return team.members.map((m) => ({
        id: m.userId,
        email: m.email,
        name: m.name,
        role: m.role,
      }));
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });

  ipcMain.handle('team:removeMember', async (_event, teamId: string, userId: string) => {
    try {
      const user = getCurrentUser();
      if (!user) {
        return { success: false, error: 'Not logged in' };
      }

      const store = readTeamsStore();
      const teamIndex = store.teams.findIndex((t) => t.id === teamId);

      if (teamIndex === -1) {
        return { success: false, error: 'Team not found' };
      }

      const team = store.teams[teamIndex];

      // Only owner or admin can remove members
      const callerMember = team.members.find((m) => m.userId === user.id);
      if (!callerMember || (callerMember.role !== 'owner' && callerMember.role !== 'admin')) {
        return { success: false, error: 'Permission denied: only owners and admins can remove members' };
      }

      // Cannot remove the owner
      const targetMember = team.members.find((m) => m.userId === userId);
      if (!targetMember) {
        return { success: false, error: 'Member not found' };
      }
      if (targetMember.role === 'owner') {
        return { success: false, error: 'Cannot remove the team owner' };
      }

      // Admins cannot remove other admins
      if (callerMember.role === 'admin' && targetMember.role === 'admin') {
        return { success: false, error: 'Admins cannot remove other admins' };
      }

      store.teams[teamIndex].members = team.members.filter((m) => m.userId !== userId);
      store.teams[teamIndex].updatedAt = new Date().toISOString();
      writeTeamsStore(store);

      return { success: true };
    } catch (err: any) {
      return { success: false, error: err.message };
    }
  });
}
