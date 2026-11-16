import { JPEG_QUALITY, LONGEST_SIDE, pictureOfAddress, reducedSize } from './galleryModel';

/** The ground a picture with transparent parts is laid on: a JPEG cannot keep transparency. */
const GROUND_OF_TRANSPARENT_PARTS = 'white';

/**
 * Reduces a picture file to what the gallery keeps: decoded by the browser,
 * drawn on a canvas at most `LONGEST_SIDE` pixels on its longest side, and
 * encoded as a JPEG. Resolves to the base64 text of the JPEG, or nothing if
 * the file could not be read as a picture.
 */
export async function reducePicture(file: Blob): Promise<string | null> {
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file);
  } catch {
    return null;
  }
  try {
    const size = reducedSize(bitmap.width, bitmap.height, LONGEST_SIDE);
    if (size === null) {
      return null;
    }
    const canvas = document.createElement('canvas');
    canvas.width = size.width;
    canvas.height = size.height;
    const context = canvas.getContext('2d');
    if (context === null) {
      return null;
    }
    context.fillStyle = GROUND_OF_TRANSPARENT_PARTS;
    context.fillRect(0, 0, size.width, size.height);
    context.imageSmoothingEnabled = true;
    context.imageSmoothingQuality = 'high';
    context.drawImage(bitmap, 0, 0, size.width, size.height);
    return pictureOfAddress(canvas.toDataURL('image/jpeg', JPEG_QUALITY));
  } finally {
    bitmap.close();
  }
}
