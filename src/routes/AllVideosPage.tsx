import { useSearchParams } from 'react-router';

import { DriveGate } from '@/components/common/DriveGate';
import { ChipBar } from '@/components/video/ChipBar';
import { DEFAULT_VIDEO_ORDER, isVideoOrder, ORDER_CHIPS } from '@/components/video/order-chips';
import { VideoFeed } from '@/components/video/VideoFeed';

export function AllVideosPage() {
  const [params, setParams] = useSearchParams();
  const sort = params.get('sort') ?? DEFAULT_VIDEO_ORDER;
  const order = isVideoOrder(sort) ? sort : DEFAULT_VIDEO_ORDER;

  return (
    <DriveGate>
      <div className="px-6">
        <h1 className="pt-6 text-4xl font-bold">All videos</h1>
        <ChipBar
          sticky
          chips={ORDER_CHIPS}
          selectedId={order}
          onSelect={(id) => setParams(id === DEFAULT_VIDEO_ORDER ? {} : { sort: id })}
        />
        <div className="pt-4 pb-12">
          <VideoFeed request={{ order }} />
        </div>
      </div>
    </DriveGate>
  );
}
