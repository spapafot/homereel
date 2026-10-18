/** Where the buffered range around the playhead ends, in seconds, for the progress bar. */
export function bufferedEndAt(video: HTMLVideoElement): number {
  const { buffered, currentTime } = video;
  for (let index = 0; index < buffered.length; index += 1) {
    if (buffered.start(index) <= currentTime && currentTime <= buffered.end(index)) {
      return buffered.end(index);
    }
  }
  return 0;
}
