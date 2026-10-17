import { useRef, useState, type FormEvent } from 'react';
import { useLocation, useNavigate, useSearchParams } from 'react-router';

import { Icon } from '@/components/common/Icon';
import { cx } from '@/lib/class-names';
import { searchPath } from '@/lib/paths';

const SEARCH_PARAM = 'search_query';

/** YouTube's top-bar search box. Submitting opens the results page. */
export function SearchBar() {
  const navigate = useNavigate();
  const location = useLocation();
  const [params] = useSearchParams();
  const inputRef = useRef<HTMLInputElement>(null);
  const activeQuery = location.pathname === '/results' ? (params.get(SEARCH_PARAM) ?? '') : '';
  const [text, setText] = useState(activeQuery);
  const [syncedQuery, setSyncedQuery] = useState(activeQuery);
  const [isFocused, setIsFocused] = useState(false);

  // Show the current results' query when navigating between searches or back to them.
  if (activeQuery !== syncedQuery) {
    setSyncedQuery(activeQuery);
    setText(activeQuery);
  }

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const query = text.trim();
    if (!query) return;
    inputRef.current?.blur();
    navigate(searchPath(query));
  };

  return (
    <form role="search" onSubmit={submit} className="flex h-10 w-full max-w-[640px]">
      <div
        className={cx(
          'flex flex-1 items-center rounded-l-full border bg-search-bg pr-1 pl-4',
          isFocused
            ? 'ml-0 border-focus shadow-[inset_0_1px_2px_rgb(0_0_0/30%)]'
            : 'ml-8 border-search-border',
        )}
      >
        {isFocused && <Icon name="search" size={20} className="mr-3 -ml-1" />}
        <input
          ref={inputRef}
          type="text"
          aria-label="Search"
          placeholder="Search"
          value={text}
          onChange={(event) => setText(event.target.value)}
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          className="h-full min-w-0 flex-1 bg-transparent text-base outline-none placeholder:text-text-secondary"
        />
        {text && (
          <button
            type="button"
            aria-label="Clear search query"
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => {
              setText('');
              inputRef.current?.focus();
            }}
            className="flex size-9 items-center justify-center rounded-full hover:bg-hover"
          >
            <Icon name="close" />
          </button>
        )}
      </div>
      <button
        type="submit"
        aria-label="Search"
        title="Search"
        className="flex w-16 items-center justify-center rounded-r-full border border-l-0 border-search-border bg-search-button hover:bg-chip"
      >
        <Icon name="search" />
      </button>
    </form>
  );
}
