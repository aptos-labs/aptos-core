import {useState} from "react";

const useExpandedList = (count: number) => {
  const [expandedList, setExpandedList] = useState<boolean[]>(
    new Array(count).fill(true),
  );

  const toggleExpandedAt = (i: number) => {
    const expandedListCopy = [...expandedList];
    expandedListCopy[i] = !expandedList[i];
    setExpandedList(expandedListCopy);
  };

  const expandAll = () => {
    setExpandedList(new Array(expandedList.length).fill(true));
  };

  const collapseAll = () => {
    setExpandedList(new Array(expandedList.length).fill(false));
  };

  return {expandedList, toggleExpandedAt, expandAll, collapseAll};
};

export default useExpandedList;
