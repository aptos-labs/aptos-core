---
title: "Set up React app"
id: "set-up-react-app"
---

# Set up a React app

We will use `react` library to build the client side with [create react app](https://create-react-app.dev/docs/getting-started#creating-an-app) (although we will explain some react decisions, we are not going to have deep dive on how react works, so we assume you have a previous experience with react).

For the UI design, we will use [Ant Design](https://ant.design/) (this is just a personal decision, you are free to use a different UI library/framework as you wish).

:::tip
Make sure you have [node and npm installed](https://nodejs.org/en/)
:::

1. On the root folder of the `my-first-dapp` project run

```js
npx create-react-app client --template typescript
```

that will create a new `client` folder in the current path.

2. Your file structure should look something like that
   ![client-folder](../../../static/img/docs/build-e2e-dapp-img-2.png)

3. `cd client`
4. `npm start`

   At this point you should have your app running on [http://localhost:3000](http://localhost:3000) and displays the default react layout.

5. On `client/src` folder we have all the react app files. Let’s clean it up a bit.
6. Open the `App.tsx` file and update its content to be

```js
function App() {
  return <div>My app goes here</div>;
}

export default App;
```

once you save the changes, you should see that the app content has changed in the browser and displays `My app goes here`.

7. Remove the `import './App.css';` and `import logo from './logo.svg';` from the `App.tsx` file. Since we remove the default imports on this file, we can remove some files in our project. Delete the files `App.css`, `logo.svg`.
8. Open the `index.tsx` file and remove the `import './index.css';` on the top of the file (line 3).
   Now you can also delete the `src/index.css` file.

### Our app UI

First we will build the App UI layout. We have 2 UI states for the app

1. When an account hasn’t created a list yet. (on the left)
2. When an account has created a list and can now add tasks to it. (on the right)
   ![dapp-ui](../../../static/img/docs/build-e2e-dapp-img-3.png)

We will use [Ant Design](https://ant.design/) library for our UI.

1. Stop the local server if running
2. On to the `client` folder Install our UI library package `npm i antd@5.1.4`
3. Update `App.tsx` with the initial state UI

```js
return (
  <>
    <Layout>
      <Row align="middle">
        <Col span={10} offset={2}>
          <h1>Our todolist</h1>
        </Col>
        <Col span={12} style={{ textAlign: "right", paddingRight: "200px" }}>
          <h1>Connect Wallet</h1>
        </Col>
      </Row>
    </Layout>
  </>
);
```

4. Dont forget to import the Components we just added

```js
import { Layout, Row, Col } from "antd";
```

5. Run the local server with `npm start`, you should see the Header that matches our UI mockup
