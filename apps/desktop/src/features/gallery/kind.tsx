import { GalleryVerticalEnd } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { GalleryView } from './GalleryView';

/** The gallery: the tasks as cards in a grid, each with a picture the view keeps. */
const gallery: Kind = {
  kind: 'gallery',
  label: 'Gallery',
  purpose: 'Tasks as pictures in a grid',
  icon: GalleryVerticalEnd,
  group: 'tasks',
  order: 100,
  component: GalleryView,
};

export default gallery;
