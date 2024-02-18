/*
 *  Worterbuch server authorization module
 *
 *  Copyright (C) 2024 Michael Bachmann
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::{auth::JwtClaims, Config};
use jsonwebtoken::{decode, DecodingKey, Validation};
use poem::{
    http::StatusCode,
    middleware::AddData,
    web::headers::{self, authorization::Bearer, HeaderMapExt},
    Endpoint, EndpointExt, Middleware, Request, Result,
};

pub struct BearerAuth {
    config: Config,
}

impl BearerAuth {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl<E: Endpoint> Middleware<E> for BearerAuth {
    type Output = BearerAuthEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        BearerAuthEndpoint {
            ep,
            config: self.config.clone(),
        }
    }
}

pub struct BearerAuthEndpoint<E> {
    ep: E,
    config: Config,
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for BearerAuthEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let jwt = req
            .headers()
            .typed_get::<headers::Authorization<Bearer>>()
            .map(|it| it.0.token().to_owned());

        if let Some(secret) = &self.config.auth_token {
            if let Some(token) = jwt {
                let token = decode::<JwtClaims>(
                    &token,
                    &DecodingKey::from_secret(secret.as_ref()),
                    &Validation::default(),
                )
                .map_err(|e| poem::Error::new(e, StatusCode::UNAUTHORIZED))?;

                (&self.ep).with(AddData::new(token.claims)).call(req).await
            } else {
                Err(poem::Error::from_string(
                    "No JWT in Auth header",
                    StatusCode::UNAUTHORIZED,
                ))
            }
        } else {
            Err(poem::Error::from_string(
                "Cannot decode JWT, no JWT secret configured",
                StatusCode::UNAUTHORIZED,
            ))
        }
    }
}

// pub(crate) struct BearerAuth {
//     wb: CloneableWbApi,
// }

// impl BearerAuth {
//     pub fn new(wb: CloneableWbApi) -> Self {
//         Self { wb }
//     }
// }

// impl<E: Endpoint> Middleware<E> for BearerAuth {
//     type Output = BearerAuthEndpoint<E>;

//     fn transform(&self, ep: E) -> Self::Output {
//         BearerAuthEndpoint {
//             ep,
//             wb: self.wb.clone(),
//         }
//     }
// }

// pub(crate) struct BearerAuthEndpoint<E> {
//     ep: E,
//     wb: CloneableWbApi,
// }

// #[poem::async_trait]
// impl<E: Endpoint> Endpoint for BearerAuthEndpoint<E> {
//     type Output = E::Output;

//     async fn call(&self, req: Request) -> Result<Self::Output> {
//         let auth_token = req
//             .headers()
//             .typed_get::<headers::Authorization<Bearer>>()
//             .map(|it| it.0.token().to_owned());
//         if self.wb.authenticate(auth_token, None).await.is_ok() {
//             self.ep.call(req).await
//         } else {
//             let mut err = Error::from_status(StatusCode::UNAUTHORIZED);
//             err.set_error_message("client failed to authenticate");
//             err.set_data(ErrorCode::AuthenticationFailed);
//             Err(err)
//         }
//     }
// }
