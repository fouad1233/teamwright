import { MessagesSquare } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { ChatView } from './ChatView';

/** The chat: the conversations with the team that are not work on a task, as one talk. */
const chat: Kind = {
  kind: 'chat',
  label: 'Chat',
  purpose: 'Talk with the team in one place',
  icon: MessagesSquare,
  group: 'pages',
  order: 160,
  component: ChatView,
};

export default chat;
