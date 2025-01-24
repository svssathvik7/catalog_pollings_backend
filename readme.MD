# Catalog pollings - Backend Documentation

## Overview

The backend for this application is developed using Rust, leveraging the Actix-web framework to create a robust and efficient RESTful API. The system integrates MongoDB as the database and uses Passkey Authentication (WebAuthn/FIDO2) along with JWT tokens for secure user management. Real-time updates are supported through Server-Sent Events (SSE).

## Features

### Core Functionalities:
- **User Authentication:** Implements modern passkey-based authentication with JWT for secure session management.
- **Poll Management:** APIs for creating, managing, voting, and closing polls.
- **Real-time Notifications:** Delivers live updates for poll results using SSE.
- **Secure Routes:** Uses custom JWT middleware to protect endpoints.

## API Endpoints and documentation

You can refer to the latest api documnetation [here](https://catalog-pollings.vercel.app/docs)

## Configuration

### Environment Variables:

- **DEV_DB_URL**: The MongoDB connection string for the development environment.
- **DEV_RP_ID**: The Relying Party (RP) ID for WebAuthn in development.
- **DEV_RP_ORIGIN**: The Relying Party (RP) origin URL for WebAuthn during development.
- **IS_DEV**: Indicates if the application is running in development mode (set to `true` for development).
- **DEV_CLIENT_ORIGIN**: The origin URL of the client in the development environment.
- **DEV_SERVER_ADDR**: The server's address in the development environment.