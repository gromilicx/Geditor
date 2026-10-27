interface User {
  id: number;
  name: string;
  isActive: boolean;
}

const users: User[] = [{ id: 1, name: "Ada", isActive: true }];

export const activeUsers = users.filter((user) => user.isActive);
