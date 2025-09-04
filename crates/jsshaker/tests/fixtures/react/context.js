// @react-jsx

import React from 'react';

export function case_provided() {
  const MyContext = React.createContext("default");

  function Inner() {
    const value = React.useContext(MyContext);
    return <div>{value}</div>;
  }

  return function main() {
    return (
      <MyContext.Provider value="hello">
        <MyContext.Provider value="world">
          <Inner />
        </MyContext.Provider>
      </MyContext.Provider>
    );
  }
}

export function case_not_provided() {
  const MyContext1 = React.createContext("default-1");
  const MyContext2 = React.createContext("default-2");

  function Inner() {
    const value = React.useContext(MyContext2);
    return <div>{value}</div>;
  }

  return function main() {
    return (
      <MyContext1.Provider value="hello">
        <Inner />
      </MyContext1.Provider>
    );
  }
}

export function case_consumed() {
  const MyContext = React.createContext("default");

  function Inner() {
    const value = React.useContext(MyContext);
    return <div>{value}</div>;
  }

  lostTrack(Inner);

  return function main() {
    return (
      <MyContext.Provider value="hello">
        <UnknownComponent />
      </MyContext.Provider>
    );
  }
}
