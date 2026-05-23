export type CommandItem = {
  id: string;
  title: string;
  command: string;
  description: string;
  favorite: boolean;
  autoEnter: boolean;
  inputMode?: "text" | "shortcut";
};

export type Category = {
  id: string;
  label: string;
  color: string;
  items: CommandItem[];
};

export type AppData = {
  appName: string;
  version: number;
  categories: Category[];
};
