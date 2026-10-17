import { Avatar } from '@/components/common/Avatar';
import { personPhotoUrl, type Person } from '@/lib/api';

/** The person's first photo in a circle, or their initial if they have none. */
export function PersonAvatar({ person, size = 40 }: { person: Person; size?: number }) {
  const photoId = person.photoIds[0];
  if (photoId == null) return <Avatar name={person.name} size={size} />;
  return (
    <img
      src={personPhotoUrl(photoId)}
      alt=""
      className="shrink-0 rounded-full bg-skeleton object-cover"
      style={{ width: size, height: size }}
    />
  );
}
