import { Fragment, type ReactNode } from 'react';

/**
 * The parts of a YouTube metadata line (channel, date…), spaced apart rather than joined with a
 * bullet, as YouTube's current site does. Empty parts are left out.
 */
export function MetadataParts({ parts }: { parts: ReactNode[] }) {
  const shown = parts.filter((part) => part != null && part !== '' && part !== false);
  return shown.map((part, index) => (
    <Fragment key={index}>
      {index > 0 && <span className="mx-1"> </span>}
      {part}
    </Fragment>
  ));
}
