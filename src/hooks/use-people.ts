import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { api, type Person, type PersonInput } from '@/lib/api';
import { toAppError } from '@/lib/app-error';
import { showToast } from '@/lib/toast';

const PEOPLE_KEY = ['people'] as const;
/** Recognized names are shown by name, so renaming or deleting someone changes them too. */
const VIDEO_PEOPLE_KEY = ['video-people'] as const;

export function usePeople() {
  return useQuery({ queryKey: PEOPLE_KEY, queryFn: api.peopleList, staleTime: Infinity });
}

/** Errors are left to the caller, which shows them in the dialog. */
export function useSavePerson() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: PersonInput) => api.personSave(input),
    onSuccess: (person, input) => {
      void queryClient.invalidateQueries({ queryKey: PEOPLE_KEY });
      void queryClient.invalidateQueries({ queryKey: VIDEO_PEOPLE_KEY });
      showToast(input.id == null ? `Added ${person.name}` : `Saved ${person.name}`);
    },
  });
}

export function useDeletePerson() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (person: Person) => api.personDelete(person.id),
    onSuccess: (_, person) => {
      void queryClient.invalidateQueries({ queryKey: PEOPLE_KEY });
      void queryClient.invalidateQueries({ queryKey: VIDEO_PEOPLE_KEY });
      showToast(`Removed ${person.name}`);
    },
    onError: (error) => showToast(toAppError(error).message),
  });
}

/** Who Gemini recognized the last time it titled the video. */
export function useVideoPeople(videoId: string) {
  return useQuery({
    queryKey: [...VIDEO_PEOPLE_KEY, videoId],
    queryFn: () => api.videoPeople(videoId),
  });
}
