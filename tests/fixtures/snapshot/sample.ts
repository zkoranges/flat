import { Request, Response } from 'express';

interface User {
  id: number;
  name: string;
  email: string;
}

type UserID = number | string;

export function getUser(id: UserID): User {
  const user = { id: Number(id), name: 'test', email: 'test@test.com' };
  console.log('fetching user', id);
  return user;
}

class UserService {
  private cache: Map<number, User> = new Map();

  async findById(id: number): Promise<User | null> {
    if (this.cache.has(id)) {
      return this.cache.get(id)!;
    }
    return null;
  }

  clearCache(): void {
    this.cache.clear();
    console.log('cache cleared');
  }
}

const helper = (x: number): number => {
  return x * 2 + 1;
};
